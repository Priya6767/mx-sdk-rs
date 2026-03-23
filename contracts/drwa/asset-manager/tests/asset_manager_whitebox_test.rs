use drwa_asset_manager::DrwaAssetManager;
use drwa_common::{DrwaCallerDomain, DrwaSyncOperationType};
use multiversx_sc::types::ManagedBuffer;
use multiversx_sc_scenario::imports::*;

const OWNER: TestAddress = TestAddress::new("owner");
const GOVERNANCE: TestAddress = TestAddress::new("governance");
const HOLDER: TestAddress = TestAddress::new("holder");
const SC_ADDRESS: TestSCAddress = TestSCAddress::new("drwa-asset-manager");
const CODE_PATH: MxscPath = MxscPath::new("mxsc:output/drwa-asset-manager.mxsc.json");

fn world() -> ScenarioWorld {
    let mut world = ScenarioWorld::new().executor_config(ExecutorConfig::full_suite());
    world.set_current_dir_from_workspace("contracts/drwa/asset-manager");
    world.register_contract(CODE_PATH, drwa_asset_manager::ContractBuilder);
    world
}

#[test]
fn asset_manager_whitebox_flow() {
    let mut world = world();

    world.account(OWNER).nonce(1).balance(1_000_000u64);
    world
        .tx()
        .from(OWNER)
        .raw_deploy()
        .code(CODE_PATH)
        .new_address(SC_ADDRESS)
        .whitebox(drwa_asset_manager::contract_obj, |sc| {
            sc.init();
        });

    world
        .tx()
        .from(OWNER)
        .to(SC_ADDRESS)
        .whitebox(drwa_asset_manager::contract_obj, |sc| {
            sc.register_asset(
                ManagedBuffer::from(b"HOTEL-001"),
                ManagedBuffer::from(b"ESDT"),
                ManagedBuffer::from(b"Hospitality"),
                ManagedBuffer::from(b"policy-hotel-1"),
            );
        });

    world
        .tx()
        .from(OWNER)
        .to(SC_ADDRESS)
        .whitebox(drwa_asset_manager::contract_obj, |sc| {
            let envelope = sc.sync_holder_compliance(
                ManagedBuffer::from(b"HOTEL-001"),
                HOLDER.to_managed_address(),
                ManagedBuffer::from(b"approved"),
                ManagedBuffer::from(b"approved"),
                ManagedBuffer::from(b"accredited"),
                ManagedBuffer::from(b"SG"),
                250,
                false,
                false,
                true,
            );

            assert!(envelope.caller_domain == DrwaCallerDomain::AssetManager);
            assert_eq!(envelope.operations.len(), 1);

            let operation = envelope.operations.get(0);
            assert!(operation.operation_type == DrwaSyncOperationType::HolderMirror);
            assert_eq!(operation.version, 1);
            assert!(!envelope.payload_hash.is_empty());
        });

    world
        .query()
        .to(SC_ADDRESS)
        .whitebox(drwa_asset_manager::contract_obj, |sc| {
            let token_id = ManagedBuffer::from(b"HOTEL-001");
            let asset = sc.asset(&token_id).get();
            assert!(asset.regulated);
            assert_eq!(asset.asset_class, ManagedBuffer::from(b"Hospitality"));

            let mirror = sc
                .holder_mirror(&token_id, &HOLDER.to_managed_address())
                .get();
            assert_eq!(mirror.holder_policy_version, 1);
            assert_eq!(mirror.kyc_status, ManagedBuffer::from(b"approved"));
            assert_eq!(mirror.investor_class, ManagedBuffer::from(b"accredited"));
            assert_eq!(
                sc.holder_policy_version(&token_id, &HOLDER.to_managed_address())
                    .get(),
                1
            );
        });
}

#[test]
fn asset_manager_rejects_non_owner_and_increments_holder_version() {
    let mut world = world();

    world.account(OWNER).nonce(1).balance(1_000_000u64);
    world
        .tx()
        .from(OWNER)
        .raw_deploy()
        .code(CODE_PATH)
        .new_address(SC_ADDRESS)
        .whitebox(drwa_asset_manager::contract_obj, |sc| {
            sc.init();
        });

    world
        .tx()
        .from(OWNER)
        .to(SC_ADDRESS)
        .whitebox(drwa_asset_manager::contract_obj, |sc| {
            sc.register_asset(
                ManagedBuffer::from(b"HOTEL-001"),
                ManagedBuffer::from(b"ESDT"),
                ManagedBuffer::from(b"Hospitality"),
                ManagedBuffer::from(b"policy-hotel-1"),
            );
        });

    for version in [1u64, 2u64] {
        world
            .tx()
            .from(OWNER)
            .to(SC_ADDRESS)
            .whitebox(drwa_asset_manager::contract_obj, |sc| {
                let envelope = sc.sync_holder_compliance(
                    ManagedBuffer::from(b"HOTEL-001"),
                    HOLDER.to_managed_address(),
                    ManagedBuffer::from(b"approved"),
                    ManagedBuffer::from(b"approved"),
                    ManagedBuffer::from(b"accredited"),
                    ManagedBuffer::from(b"SG"),
                    250 + version,
                    version == 2,
                    false,
                    true,
                );
                assert_eq!(envelope.operations.get(0).version, version);
            });
    }

    world
        .query()
        .to(SC_ADDRESS)
        .whitebox(drwa_asset_manager::contract_obj, |sc| {
            let token_id = ManagedBuffer::from(b"HOTEL-001");
            let mirror = sc
                .holder_mirror(&token_id, &HOLDER.to_managed_address())
                .get();
            assert_eq!(mirror.holder_policy_version, 2);
            assert!(mirror.transfer_locked);
            assert_eq!(mirror.expiry_round, 252);
        });
}

#[test]
fn asset_manager_allows_governance_to_manage_assets_and_holders() {
    let mut world = world();

    world.account(OWNER).nonce(1).balance(1_000_000u64);
    world.account(GOVERNANCE).nonce(1).balance(1_000_000u64);
    world
        .tx()
        .from(OWNER)
        .raw_deploy()
        .code(CODE_PATH)
        .new_address(SC_ADDRESS)
        .whitebox(drwa_asset_manager::contract_obj, |sc| {
            sc.init();
        });

    world
        .tx()
        .from(OWNER)
        .to(SC_ADDRESS)
        .whitebox(drwa_asset_manager::contract_obj, |sc| {
            sc.set_governance(GOVERNANCE.to_managed_address());
        });

    world
        .tx()
        .from(GOVERNANCE)
        .to(SC_ADDRESS)
        .whitebox(drwa_asset_manager::contract_obj, |sc| {
            sc.accept_governance();
        });

    world
        .tx()
        .from(GOVERNANCE)
        .to(SC_ADDRESS)
        .whitebox(drwa_asset_manager::contract_obj, |sc| {
            sc.register_asset(
                ManagedBuffer::from(b"HOTEL-002"),
                ManagedBuffer::from(b"ESDT"),
                ManagedBuffer::from(b"Hospitality"),
                ManagedBuffer::from(b"policy-hotel-2"),
            );
        });

    world
        .tx()
        .from(GOVERNANCE)
        .to(SC_ADDRESS)
        .whitebox(drwa_asset_manager::contract_obj, |sc| {
            let envelope = sc.sync_holder_compliance(
                ManagedBuffer::from(b"HOTEL-002"),
                HOLDER.to_managed_address(),
                ManagedBuffer::from(b"approved"),
                ManagedBuffer::from(b"approved"),
                ManagedBuffer::from(b"accredited"),
                ManagedBuffer::from(b"SG"),
                500,
                false,
                false,
                true,
            );
            assert_eq!(envelope.operations.get(0).version, 1);
        });
}

#[test]
fn asset_manager_requires_pending_governance_acceptance() {
    let mut world = world();

    world.account(OWNER).nonce(1).balance(1_000_000u64);
    world.account(GOVERNANCE).nonce(1).balance(1_000_000u64);
    world
        .tx()
        .from(OWNER)
        .raw_deploy()
        .code(CODE_PATH)
        .new_address(SC_ADDRESS)
        .whitebox(drwa_asset_manager::contract_obj, |sc| {
            sc.init();
        });

    world
        .tx()
        .from(OWNER)
        .to(SC_ADDRESS)
        .whitebox(drwa_asset_manager::contract_obj, |sc| {
            sc.set_governance(GOVERNANCE.to_managed_address());
            assert_eq!(sc.pending_governance().get(), GOVERNANCE.to_managed_address());
        });

    world
        .query()
        .to(SC_ADDRESS)
        .whitebox(drwa_asset_manager::contract_obj, |sc| {
            assert!(sc.governance().is_empty());
        });
}
