#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub type PublicId<M> = ManagedBuffer<M>;

#[type_abi]
#[derive(
    TopEncode,
    TopDecode,
    NestedEncode,
    NestedDecode,
    ManagedVecItem,
    Clone,
    PartialEq,
    Eq,
)]
pub struct MrvReportProof<M: ManagedTypeApi> {
    pub report_id: PublicId<M>,
    pub public_tenant_id: PublicId<M>,
    pub public_farm_id: PublicId<M>,
    pub public_season_id: PublicId<M>,
    pub report_hash: ManagedBuffer<M>,
    pub hash_algo: ManagedBuffer<M>,
    pub canonicalization: ManagedBuffer<M>,
    pub methodology_version: u64,
    pub anchored_at: u64,
}
