use drwa_attestation::DrwaAttestation;
use multiversx_sc::types::ManagedBuffer;
use multiversx_sc_scenario::imports::*;

const OWNER: TestAddress = TestAddress::new("owner");
const AUDITOR: TestAddress = TestAddress::new("auditor");
const SUBJECT: TestAddress = TestAddress::new("subject");
const OTHER: TestAddress = TestAddress::new("other");
const SC_ADDRESS: TestSCAddress = TestSCAddress::new("drwa-attestation");
const CODE_PATH: MxscPath = MxscPath::new("mxsc:output/drwa-attestation.mxsc.json");

fn world() -> ScenarioWorld {
    let mut world = ScenarioWorld::new().executor_config(ExecutorConfig::full_suite());
    world.set_current_dir_from_workspace("contracts/drwa/attestation");
    world.register_contract(CODE_PATH, drwa_attestation::ContractBuilder);
    world
}

#[test]
fn attestation_whitebox_flow() {
    let mut world = world();

    world.account(OWNER).nonce(1).balance(1_000_000u64);
    world.account(AUDITOR).nonce(1).balance(1_000_000u64);
    world.account(OTHER).nonce(1).balance(1_000_000u64);

    world
        .tx()
        .from(OWNER)
        .raw_deploy()
        .code(CODE_PATH)
        .new_address(SC_ADDRESS)
        .whitebox(drwa_attestation::contract_obj, |sc| {
            sc.init(AUDITOR.to_managed_address());
        });

    world
        .tx()
        .from(AUDITOR)
        .to(SC_ADDRESS)
        .whitebox(drwa_attestation::contract_obj, |sc| {
            sc.record_attestation(
                ManagedBuffer::from(b"CARBON-001"),
                SUBJECT.to_managed_address(),
                ManagedBuffer::from(b"MRV"),
                ManagedBuffer::from(b"hash-001"),
                true,
            );
        });

    world
        .query()
        .to(SC_ADDRESS)
        .whitebox(drwa_attestation::contract_obj, |sc| {
            let record = sc
                .attestation(&ManagedBuffer::from(b"CARBON-001"), &SUBJECT.to_managed_address())
                .get();
            assert_eq!(record.token_id, ManagedBuffer::from(b"CARBON-001"));
            assert_eq!(record.attestation_type, ManagedBuffer::from(b"MRV"));
            assert_eq!(record.evidence_hash, ManagedBuffer::from(b"hash-001"));
            assert!(record.approved);
        });
}

#[test]
fn attestation_rejects_non_auditor() {
    let mut world = world();

    world.account(OWNER).nonce(1).balance(1_000_000u64);
    world.account(AUDITOR).nonce(1).balance(1_000_000u64);
    world.account(OTHER).nonce(1).balance(1_000_000u64);

    world
        .tx()
        .from(OWNER)
        .raw_deploy()
        .code(CODE_PATH)
        .new_address(SC_ADDRESS)
        .whitebox(drwa_attestation::contract_obj, |sc| {
            sc.init(AUDITOR.to_managed_address());
        });

    world
        .tx()
        .from(OTHER)
        .to(SC_ADDRESS)
        .returns(ExpectError(4u64, "caller not auditor"))
        .whitebox(drwa_attestation::contract_obj, |sc| {
            sc.record_attestation(
                ManagedBuffer::from(b"CARBON-001"),
                SUBJECT.to_managed_address(),
                ManagedBuffer::from(b"MRV"),
                ManagedBuffer::from(b"hash-001"),
                true,
            );
        });
}

#[test]
fn attestation_owner_can_rotate_auditor() {
    let mut world = world();

    world.account(OWNER).nonce(1).balance(1_000_000u64);
    world.account(AUDITOR).nonce(1).balance(1_000_000u64);
    world.account(OTHER).nonce(1).balance(1_000_000u64);

    world
        .tx()
        .from(OWNER)
        .raw_deploy()
        .code(CODE_PATH)
        .new_address(SC_ADDRESS)
        .whitebox(drwa_attestation::contract_obj, |sc| {
            sc.init(AUDITOR.to_managed_address());
        });

    world
        .tx()
        .from(OWNER)
        .to(SC_ADDRESS)
        .whitebox(drwa_attestation::contract_obj, |sc| {
            sc.set_auditor(OTHER.to_managed_address());
            assert_eq!(sc.pending_auditor().get(), OTHER.to_managed_address());
        });

    world
        .tx()
        .from(OTHER)
        .to(SC_ADDRESS)
        .whitebox(drwa_attestation::contract_obj, |sc| {
            sc.accept_auditor();
        });

    world
        .tx()
        .from(AUDITOR)
        .to(SC_ADDRESS)
        .returns(ExpectError(4u64, "caller not auditor"))
        .whitebox(drwa_attestation::contract_obj, |sc| {
            sc.record_attestation(
                ManagedBuffer::from(b"CARBON-001"),
                SUBJECT.to_managed_address(),
                ManagedBuffer::from(b"MRV"),
                ManagedBuffer::from(b"hash-001"),
                true,
            );
        });

    world
        .tx()
        .from(OTHER)
        .to(SC_ADDRESS)
        .whitebox(drwa_attestation::contract_obj, |sc| {
            sc.record_attestation(
                ManagedBuffer::from(b"CARBON-001"),
                SUBJECT.to_managed_address(),
                ManagedBuffer::from(b"MRV"),
                ManagedBuffer::from(b"hash-rotated"),
                true,
            );
        });
}

#[test]
fn attestation_requires_pending_auditor_acceptance() {
    let mut world = world();

    world.account(OWNER).nonce(1).balance(1_000_000u64);
    world.account(AUDITOR).nonce(1).balance(1_000_000u64);
    world.account(OTHER).nonce(1).balance(1_000_000u64);

    world
        .tx()
        .from(OWNER)
        .raw_deploy()
        .code(CODE_PATH)
        .new_address(SC_ADDRESS)
        .whitebox(drwa_attestation::contract_obj, |sc| {
            sc.init(AUDITOR.to_managed_address());
        });

    world
        .tx()
        .from(OWNER)
        .to(SC_ADDRESS)
        .whitebox(drwa_attestation::contract_obj, |sc| {
            sc.set_auditor(OTHER.to_managed_address());
        });

    world
        .query()
        .to(SC_ADDRESS)
        .whitebox(drwa_attestation::contract_obj, |sc| {
            assert_eq!(sc.auditor().get(), AUDITOR.to_managed_address());
            assert_eq!(sc.pending_auditor().get(), OTHER.to_managed_address());
        });
}
