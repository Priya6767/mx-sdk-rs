#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub type TokenId<M> = ManagedBuffer<M>;
pub type HolderId<M> = ManagedAddress<M>;

#[type_abi]
#[derive(
    TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem, Clone, PartialEq, Eq,
)]
pub enum DrwaSyncOperationType {
    TokenPolicy,
    HolderMirror,
    HolderMirrorDelete,
}

#[type_abi]
#[derive(
    TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem, Clone, PartialEq, Eq,
)]
pub enum DrwaCallerDomain {
    PolicyRegistry,
    AssetManager,
    RecoveryAdmin,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem, Clone)]
pub struct DrwaTokenPolicy<M: ManagedTypeApi> {
    pub drwa_enabled: bool,
    pub global_pause: bool,
    pub strict_auditor_mode: bool,
    pub metadata_protection_enabled: bool,
    pub token_policy_version: u64,
    pub allowed_investor_classes: ManagedVec<M, ManagedBuffer<M>>,
    pub allowed_jurisdictions: ManagedVec<M, ManagedBuffer<M>>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem, Clone)]
pub struct DrwaHolderMirror<M: ManagedTypeApi> {
    pub holder_policy_version: u64,
    pub kyc_status: ManagedBuffer<M>,
    pub aml_status: ManagedBuffer<M>,
    pub investor_class: ManagedBuffer<M>,
    pub jurisdiction_code: ManagedBuffer<M>,
    pub expiry_round: u64,
    pub transfer_locked: bool,
    pub receive_locked: bool,
    pub auditor_authorized: bool,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem, Clone)]
pub struct DrwaSyncOperation<M: ManagedTypeApi> {
    pub operation_type: DrwaSyncOperationType,
    pub token_id: ManagedBuffer<M>,
    pub holder: ManagedAddress<M>,
    pub version: u64,
    pub body: ManagedBuffer<M>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct DrwaSyncEnvelope<M: ManagedTypeApi> {
    pub caller_domain: DrwaCallerDomain,
    pub payload_hash: ManagedBuffer<M>,
    pub operations: ManagedVec<M, DrwaSyncOperation<M>>,
}

pub fn serialize_sync_envelope_payload<M: ManagedTypeApi>(
    caller_domain: &DrwaCallerDomain,
    operations: &ManagedVec<M, DrwaSyncOperation<M>>,
) -> ManagedBuffer<M> {
    let mut result = ManagedBuffer::new();
    let caller_tag = match caller_domain {
        DrwaCallerDomain::PolicyRegistry => 0u8,
        DrwaCallerDomain::AssetManager => 1u8,
        DrwaCallerDomain::RecoveryAdmin => 2u8,
    };
    result.append_bytes(&[caller_tag]);

    for operation in operations.iter() {
        let op_tag = match operation.operation_type {
            DrwaSyncOperationType::TokenPolicy => 0u8,
            DrwaSyncOperationType::HolderMirror => 1u8,
            DrwaSyncOperationType::HolderMirrorDelete => 2u8,
        };
        result.append_bytes(&[op_tag]);
        push_len_prefixed(&mut result, &operation.token_id);
        push_len_prefixed(&mut result, &operation.holder.as_managed_buffer());
        result.append_bytes(&operation.version.to_be_bytes());
        push_len_prefixed(&mut result, &operation.body);
    }

    result
}
fn push_len_prefixed<M: ManagedTypeApi>(dest: &mut ManagedBuffer<M>, value: &ManagedBuffer<M>) {
    let len = value.len() as u32;
    dest.append_bytes(&len.to_be_bytes());
    dest.append(value);
}

/// Builds the binary hook payload passed to `managedDRWASyncMirror`.
/// Format: [32-byte keccak256 payload_hash] || [canonical binary payload].
/// The Go-side decoder (decodeDRWASyncEnvelope) detects the binary path by
/// checking that the first byte is not '{', then splits at offset 32.
pub fn build_sync_hook_payload<M: ManagedTypeApi>(
    caller_domain: &DrwaCallerDomain,
    operations: &ManagedVec<M, DrwaSyncOperation<M>>,
    payload_hash: &ManagedBuffer<M>,
) -> ManagedBuffer<M> {
    let canonical_payload = serialize_sync_envelope_payload(caller_domain, operations);
    let mut result = ManagedBuffer::new();
    result.append(payload_hash);
    result.append(&canonical_payload);
    result
}
