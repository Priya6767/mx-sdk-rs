#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use multiversx_sc::api::HandleConstraints;

use drwa_common::{
    build_sync_hook_payload, DrwaCallerDomain, DrwaHolderMirror, DrwaSyncEnvelope,
    DrwaSyncOperation, DrwaSyncOperationType, serialize_sync_envelope_payload,
};

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    fn managedDRWASyncMirror(payloadHandle: i32) -> i32;
}

#[inline]
fn invoke_drwa_sync_hook(payload_handle: i32) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        managedDRWASyncMirror(payload_handle);
    }

    #[cfg(not(target_arch = "wasm32"))]
    let _ = payload_handle;
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem, Clone)]
pub struct AssetRecord<M: ManagedTypeApi> {
    pub token_id: ManagedBuffer<M>,
    pub carrier_type: ManagedBuffer<M>,
    pub asset_class: ManagedBuffer<M>,
    pub policy_id: ManagedBuffer<M>,
    pub regulated: bool,
}

#[multiversx_sc::contract]
pub trait DrwaAssetManager {
    #[init]
    fn init(&self) {}

    #[only_owner]
    #[endpoint(setGovernance)]
    fn set_governance(&self, governance: ManagedAddress) {
        require!(!governance.is_zero(), "governance must not be zero");
        self.pending_governance().set(&governance);
        self.drwa_governance_proposed_event(&governance);
    }

    #[endpoint(acceptGovernance)]
    fn accept_governance(&self) {
        require!(
            !self.pending_governance().is_empty(),
            "pending governance not set"
        );

        let caller = self.blockchain().get_caller();
        let pending = self.pending_governance().get();
        require!(caller == pending, "caller not pending governance");

        self.governance().set(&pending);
        self.pending_governance().clear();
        self.drwa_governance_accepted_event(&pending);
    }

    #[endpoint(registerAsset)]
    fn register_asset(
        &self,
        token_id: ManagedBuffer,
        carrier_type: ManagedBuffer,
        asset_class: ManagedBuffer,
        policy_id: ManagedBuffer,
    ) {
        self.require_governance_or_owner();

        self.asset(&token_id).set(AssetRecord {
            token_id: token_id.clone(),
            carrier_type,
            asset_class,
            policy_id: policy_id.clone(),
            regulated: true,
        });
        self.drwa_asset_registered_event(&token_id, &policy_id, true);
    }

    #[endpoint(syncHolderCompliance)]
    fn sync_holder_compliance(
        &self,
        token_id: ManagedBuffer,
        holder: ManagedAddress,
        kyc_status: ManagedBuffer,
        aml_status: ManagedBuffer,
        investor_class: ManagedBuffer,
        jurisdiction_code: ManagedBuffer,
        expiry_round: u64,
        transfer_locked: bool,
        receive_locked: bool,
        auditor_authorized: bool,
    ) -> DrwaSyncEnvelope<Self::Api> {
        self.require_governance_or_owner();

        let next_version = self.holder_policy_version(&token_id, &holder).get() + 1;

        let mirror = DrwaHolderMirror {
            holder_policy_version: next_version,
            kyc_status,
            aml_status,
            investor_class,
            jurisdiction_code,
            expiry_round,
            transfer_locked,
            receive_locked,
            auditor_authorized,
        };

        self.holder_mirror(&token_id, &holder).set(mirror.clone());
        self.holder_policy_version(&token_id, &holder)
            .set(next_version);

        let body = self.serialize_holder(&mirror);
        let mut operations = ManagedVec::new();
        operations.push(DrwaSyncOperation {
            operation_type: DrwaSyncOperationType::HolderMirror,
            token_id: token_id.clone(),
            holder: holder.clone(),
            version: next_version,
            body,
        });

        let caller_domain = DrwaCallerDomain::AssetManager;
        let payload_hash = self
            .crypto()
            .keccak256(&serialize_sync_envelope_payload(
                &caller_domain,
                &operations,
            ))
            .as_managed_buffer()
            .clone();

        let hook_payload =
            build_sync_hook_payload(&caller_domain, &operations, &payload_hash);
        invoke_drwa_sync_hook(hook_payload.get_handle().get_raw_handle());

        DrwaSyncEnvelope {
            caller_domain,
            payload_hash,
            operations,
        }
    }

    #[view(getAsset)]
    #[storage_mapper("asset")]
    fn asset(&self, token_id: &ManagedBuffer) -> SingleValueMapper<AssetRecord<Self::Api>>;

    #[storage_mapper("holderMirror")]
    fn holder_mirror(
        &self,
        token_id: &ManagedBuffer,
        holder: &ManagedAddress,
    ) -> SingleValueMapper<DrwaHolderMirror<Self::Api>>;

    #[storage_mapper("holderPolicyVersion")]
    fn holder_policy_version(
        &self,
        token_id: &ManagedBuffer,
        holder: &ManagedAddress,
    ) -> SingleValueMapper<u64>;

    #[view(getGovernance)]
    #[storage_mapper("governance")]
    fn governance(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getPendingGovernance)]
    #[storage_mapper("pendingGovernance")]
    fn pending_governance(&self) -> SingleValueMapper<ManagedAddress>;

    #[event("drwaAssetRegistered")]
    fn drwa_asset_registered_event(
        &self,
        #[indexed] token_id: &ManagedBuffer,
        #[indexed] policy_id: &ManagedBuffer,
        #[indexed] regulated: bool,
    );

    #[event("drwaGovernanceProposed")]
    fn drwa_governance_proposed_event(&self, #[indexed] governance: &ManagedAddress);

    #[event("drwaGovernanceAccepted")]
    fn drwa_governance_accepted_event(&self, #[indexed] governance: &ManagedAddress);

    fn serialize_holder(&self, holder: &DrwaHolderMirror<Self::Api>) -> ManagedBuffer {
        let mut result = ManagedBuffer::new();
        result.append_bytes(&holder.holder_policy_version.to_be_bytes());
        append_len_prefixed(&mut result, &holder.kyc_status);
        append_len_prefixed(&mut result, &holder.aml_status);
        append_len_prefixed(&mut result, &holder.investor_class);
        append_len_prefixed(&mut result, &holder.jurisdiction_code);
        result.append_bytes(&holder.expiry_round.to_be_bytes());
        result.append_bytes(&[holder.transfer_locked as u8]);
        result.append_bytes(&[holder.receive_locked as u8]);
        result.append_bytes(&[holder.auditor_authorized as u8]);
        result
    }

    fn require_governance_or_owner(&self) {
        let caller = self.blockchain().get_caller();
        if !self.governance().is_empty() && caller == self.governance().get() {
            return;
        }

        require!(
            caller == self.blockchain().get_owner_address(),
            "caller not authorized"
        );
    }
}

fn append_len_prefixed<M: ManagedTypeApi>(dest: &mut ManagedBuffer<M>, value: &ManagedBuffer<M>) {
    let len = value.len() as u32;
    dest.append_bytes(&len.to_be_bytes());
    dest.append(value);
}
