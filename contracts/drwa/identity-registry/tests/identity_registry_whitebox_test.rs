use drwa_identity_registry::DrwaIdentityRegistry;
use multiversx_sc::types::ManagedBuffer;
use multiversx_sc_scenario::imports::*;

const OWNER: TestAddress = TestAddress::new("owner");
const GOVERNANCE: TestAddress = TestAddress::new("governance");
const ISSUER: TestAddress = TestAddress::new("issuer");
const INTRUDER: TestAddress = TestAddress::new("intruder");
const SC_ADDRESS: TestSCAddress = TestSCAddress::new("drwa-identity-registry");
const CODE_PATH: MxscPath = MxscPath::new("mxsc:output/drwa-identity-registry.mxsc.json");

fn world() -> ScenarioWorld {
    let mut world = ScenarioWorld::new().executor_config(ExecutorConfig::full_suite());
    world.set_current_dir_from_workspace("contracts/drwa/identity-registry");
    world.register_contract(CODE_PATH, drwa_identity_registry::ContractBuilder);
    world
}

#[test]
fn identity_registry_whitebox_flow() {
    let mut world = world();

    world.account(OWNER).nonce(1).balance(1_000_000u64);
    world.account(GOVERNANCE).nonce(1).balance(1_000_000u64);
    world.account(INTRUDER).nonce(1).balance(1_000_000u64);

    world
        .tx()
        .from(OWNER)
        .raw_deploy()
        .code(CODE_PATH)
        .new_address(SC_ADDRESS)
        .whitebox(drwa_identity_registry::contract_obj, |sc| {
            sc.init(GOVERNANCE.to_managed_address());
        });

    world.tx().from(GOVERNANCE).to(SC_ADDRESS).whitebox(
        drwa_identity_registry::contract_obj,
        |sc| {
            sc.register_identity(
                ISSUER.to_managed_address(),
                ManagedBuffer::from(b"Carbon Ventures"),
                ManagedBuffer::from(b"SG"),
                ManagedBuffer::from(b"REG-001"),
                ManagedBuffer::from(b"SPV"),
            );
        },
    );

    world.tx().from(GOVERNANCE).to(SC_ADDRESS).whitebox(
        drwa_identity_registry::contract_obj,
        |sc| {
            sc.update_compliance_status(
                ISSUER.to_managed_address(),
                ManagedBuffer::from(b"approved"),
                ManagedBuffer::from(b"approved"),
                ManagedBuffer::from(b"issuer"),
                100,
            );
        },
    );

    world
        .query()
        .to(SC_ADDRESS)
        .whitebox(drwa_identity_registry::contract_obj, |sc| {
            let record = sc.identity(&ISSUER.to_managed_address()).get();
            assert_eq!(record.legal_name, ManagedBuffer::from(b"Carbon Ventures"));
            assert_eq!(record.jurisdiction_code, ManagedBuffer::from(b"SG"));
            assert_eq!(record.registration_number, ManagedBuffer::from(b"REG-001"));
            assert_eq!(record.entity_type, ManagedBuffer::from(b"SPV"));
            assert_eq!(record.kyc_status, ManagedBuffer::from(b"approved"));
            assert_eq!(record.aml_status, ManagedBuffer::from(b"approved"));
            assert_eq!(record.investor_class, ManagedBuffer::from(b"issuer"));
            assert_eq!(record.expiry_round, 100);
        });
}

#[test]
fn identity_registry_registration_sets_future_expiry() {
    let mut world = world();

    world.account(OWNER).nonce(1).balance(1_000_000u64);
    world.account(GOVERNANCE).nonce(1).balance(1_000_000u64);

    world
        .tx()
        .from(OWNER)
        .raw_deploy()
        .code(CODE_PATH)
        .new_address(SC_ADDRESS)
        .whitebox(drwa_identity_registry::contract_obj, |sc| {
            sc.init(GOVERNANCE.to_managed_address());
        });

    world.tx().from(GOVERNANCE).to(SC_ADDRESS).whitebox(
        drwa_identity_registry::contract_obj,
        |sc| {
            sc.register_identity(
                ISSUER.to_managed_address(),
                ManagedBuffer::from(b"Carbon Ventures"),
                ManagedBuffer::from(b"SG"),
                ManagedBuffer::from(b"REG-001"),
                ManagedBuffer::from(b"SPV"),
            );
        },
    );

    world
        .query()
        .to(SC_ADDRESS)
        .whitebox(drwa_identity_registry::contract_obj, |sc| {
            let record = sc.identity(&ISSUER.to_managed_address()).get();
            assert_eq!(record.expiry_round, 10_000);
        });
}

#[test]
fn identity_registry_rejects_unauthorized_update() {
    let mut world = world();

    world.account(OWNER).nonce(1).balance(1_000_000u64);
    world.account(GOVERNANCE).nonce(1).balance(1_000_000u64);
    world.account(INTRUDER).nonce(1).balance(1_000_000u64);

    world
        .tx()
        .from(OWNER)
        .raw_deploy()
        .code(CODE_PATH)
        .new_address(SC_ADDRESS)
        .whitebox(drwa_identity_registry::contract_obj, |sc| {
            sc.init(GOVERNANCE.to_managed_address());
        });

    world
        .tx()
        .from(INTRUDER)
        .to(SC_ADDRESS)
        .returns(ExpectError(4u64, "caller not authorized"))
        .whitebox(drwa_identity_registry::contract_obj, |sc| {
            sc.register_identity(
                ISSUER.to_managed_address(),
                ManagedBuffer::from(b"Blocked"),
                ManagedBuffer::from(b"US"),
                ManagedBuffer::from(b"REG-X"),
                ManagedBuffer::from(b"SPV"),
            );
        });
}

#[test]
fn identity_registry_requires_pending_governance_acceptance() {
    let mut world = world();

    world.account(OWNER).nonce(1).balance(1_000_000u64);
    world.account(GOVERNANCE).nonce(1).balance(1_000_000u64);
    world.account(INTRUDER).nonce(1).balance(1_000_000u64);

    world
        .tx()
        .from(OWNER)
        .raw_deploy()
        .code(CODE_PATH)
        .new_address(SC_ADDRESS)
        .whitebox(drwa_identity_registry::contract_obj, |sc| {
            sc.init(INTRUDER.to_managed_address());
        });

    world
        .tx()
        .from(OWNER)
        .to(SC_ADDRESS)
        .whitebox(drwa_identity_registry::contract_obj, |sc| {
            sc.set_governance(GOVERNANCE.to_managed_address());
            assert_eq!(sc.pending_governance().get(), GOVERNANCE.to_managed_address());
        });

    world
        .query()
        .to(SC_ADDRESS)
        .whitebox(drwa_identity_registry::contract_obj, |sc| {
            assert_eq!(sc.governance().get(), INTRUDER.to_managed_address());
        });

    world
        .tx()
        .from(GOVERNANCE)
        .to(SC_ADDRESS)
        .whitebox(drwa_identity_registry::contract_obj, |sc| {
            sc.accept_governance();
        });

    world
        .query()
        .to(SC_ADDRESS)
        .whitebox(drwa_identity_registry::contract_obj, |sc| {
            assert_eq!(sc.governance().get(), GOVERNANCE.to_managed_address());
        });
}
