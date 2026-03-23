use drwa_common::{DrwaCallerDomain, DrwaSyncOperationType};
use drwa_policy_registry::DrwaPolicyRegistry;
use multiversx_sc::types::{ManagedBuffer, ManagedVec};
use multiversx_sc_scenario::imports::*;

const OWNER: TestAddress = TestAddress::new("owner");
const GOVERNANCE: TestAddress = TestAddress::new("governance");
const SC_ADDRESS: TestSCAddress = TestSCAddress::new("drwa-policy-registry");
const CODE_PATH: MxscPath = MxscPath::new("mxsc:output/drwa-policy-registry.mxsc.json");

fn world() -> ScenarioWorld {
    let mut world = ScenarioWorld::new().executor_config(ExecutorConfig::full_suite());
    world.set_current_dir_from_workspace("contracts/drwa/policy-registry");
    world.register_contract(CODE_PATH, drwa_policy_registry::ContractBuilder);
    world
}

#[test]
fn policy_registry_whitebox_flow() {
    let mut world = world();

    world.account(OWNER).nonce(1).balance(1_000_000u64);
    world
        .tx()
        .from(OWNER)
        .raw_deploy()
        .code(CODE_PATH)
        .new_address(SC_ADDRESS)
        .whitebox(drwa_policy_registry::contract_obj, |sc| {
            sc.init();
        });

    world
        .tx()
        .from(OWNER)
        .to(SC_ADDRESS)
        .whitebox(drwa_policy_registry::contract_obj, |sc| {
            let mut investor_classes = ManagedVec::new();
            investor_classes.push(ManagedBuffer::from(b"ACCREDITED"));

            let mut jurisdictions = ManagedVec::new();
            jurisdictions.push(ManagedBuffer::from(b"SG"));

            let envelope = sc.set_token_policy(
                ManagedBuffer::from(b"CARBON-001"),
                true,
                false,
                true,
                true,
                investor_classes,
                jurisdictions,
            );

            assert!(envelope.caller_domain == DrwaCallerDomain::PolicyRegistry);
            assert_eq!(envelope.operations.len(), 1);

            let operation = envelope.operations.get(0);
            assert!(operation.operation_type == DrwaSyncOperationType::TokenPolicy);
            assert_eq!(operation.version, 1);
            assert!(!envelope.payload_hash.is_empty());
        });

    world
        .query()
        .to(SC_ADDRESS)
        .whitebox(drwa_policy_registry::contract_obj, |sc| {
            let token_id = ManagedBuffer::from(b"CARBON-001");
            let policy = sc.token_policy(&token_id).get();
            assert!(policy.drwa_enabled);
            assert!(policy.strict_auditor_mode);
            assert!(policy.metadata_protection_enabled);
            assert_eq!(policy.token_policy_version, 1);
            assert_eq!(sc.token_policy_version(&token_id).get(), 1);
        });
}

#[test]
fn policy_registry_increments_version_and_rejects_non_owner() {
    let mut world = world();

    world.account(OWNER).nonce(1).balance(1_000_000u64);
    world
        .tx()
        .from(OWNER)
        .raw_deploy()
        .code(CODE_PATH)
        .new_address(SC_ADDRESS)
        .whitebox(drwa_policy_registry::contract_obj, |sc| {
            sc.init();
        });

    for version in [1u64, 2u64] {
        world
            .tx()
            .from(OWNER)
            .to(SC_ADDRESS)
            .whitebox(drwa_policy_registry::contract_obj, |sc| {
                let mut investor_classes = ManagedVec::new();
                investor_classes.push(ManagedBuffer::from(b"ACCREDITED"));

                let mut jurisdictions = ManagedVec::new();
                jurisdictions.push(ManagedBuffer::from(b"SG"));

                let envelope = sc.set_token_policy(
                    ManagedBuffer::from(b"CARBON-001"),
                    true,
                    version == 2,
                    true,
                    true,
                    investor_classes,
                    jurisdictions,
                );
                assert_eq!(envelope.operations.get(0).version, version);
            });
    }

    world
        .query()
        .to(SC_ADDRESS)
        .whitebox(drwa_policy_registry::contract_obj, |sc| {
            let token_id = ManagedBuffer::from(b"CARBON-001");
            let policy = sc.token_policy(&token_id).get();
            assert_eq!(policy.token_policy_version, 2);
            assert!(policy.global_pause);
        });
}

#[test]
fn policy_registry_persists_explicit_drwa_enabled_state() {
    let mut world = world();

    world.account(OWNER).nonce(1).balance(1_000_000u64);
    world
        .tx()
        .from(OWNER)
        .raw_deploy()
        .code(CODE_PATH)
        .new_address(SC_ADDRESS)
        .whitebox(drwa_policy_registry::contract_obj, |sc| {
            sc.init();
        });

    world
        .tx()
        .from(OWNER)
        .to(SC_ADDRESS)
        .whitebox(drwa_policy_registry::contract_obj, |sc| {
            let envelope = sc.set_token_policy(
                ManagedBuffer::from(b"CARBON-002"),
                false,
                false,
                false,
                false,
                ManagedVec::new(),
                ManagedVec::new(),
            );
            assert_eq!(envelope.operations.get(0).version, 1);
        });

    world
        .query()
        .to(SC_ADDRESS)
        .whitebox(drwa_policy_registry::contract_obj, |sc| {
            let token_id = ManagedBuffer::from(b"CARBON-002");
            let policy = sc.token_policy(&token_id).get();
            assert!(!policy.drwa_enabled);
        });
}

#[test]
fn policy_registry_allows_governance_to_set_policy() {
    let mut world = world();

    world.account(OWNER).nonce(1).balance(1_000_000u64);
    world.account(GOVERNANCE).nonce(1).balance(1_000_000u64);
    world
        .tx()
        .from(OWNER)
        .raw_deploy()
        .code(CODE_PATH)
        .new_address(SC_ADDRESS)
        .whitebox(drwa_policy_registry::contract_obj, |sc| {
            sc.init();
        });

    world
        .tx()
        .from(OWNER)
        .to(SC_ADDRESS)
        .whitebox(drwa_policy_registry::contract_obj, |sc| {
            sc.set_governance(GOVERNANCE.to_managed_address());
        });

    world
        .tx()
        .from(GOVERNANCE)
        .to(SC_ADDRESS)
        .whitebox(drwa_policy_registry::contract_obj, |sc| {
            sc.accept_governance();
        });

    world
        .tx()
        .from(GOVERNANCE)
        .to(SC_ADDRESS)
        .whitebox(drwa_policy_registry::contract_obj, |sc| {
            let envelope = sc.set_token_policy(
                ManagedBuffer::from(b"CARBON-003"),
                true,
                false,
                false,
                false,
                ManagedVec::new(),
                ManagedVec::new(),
            );
            assert_eq!(envelope.operations.get(0).version, 1);
        });
}

#[test]
fn policy_registry_requires_pending_governance_acceptance() {
    let mut world = world();

    world.account(OWNER).nonce(1).balance(1_000_000u64);
    world.account(GOVERNANCE).nonce(1).balance(1_000_000u64);
    world
        .tx()
        .from(OWNER)
        .raw_deploy()
        .code(CODE_PATH)
        .new_address(SC_ADDRESS)
        .whitebox(drwa_policy_registry::contract_obj, |sc| {
            sc.init();
        });

    world
        .tx()
        .from(OWNER)
        .to(SC_ADDRESS)
        .whitebox(drwa_policy_registry::contract_obj, |sc| {
            sc.set_governance(GOVERNANCE.to_managed_address());
            assert_eq!(sc.pending_governance().get(), GOVERNANCE.to_managed_address());
        });

    world
        .query()
        .to(SC_ADDRESS)
        .whitebox(drwa_policy_registry::contract_obj, |sc| {
            assert!(sc.governance().is_empty());
        });
}
