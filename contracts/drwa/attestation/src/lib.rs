#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem, Clone)]
pub struct AttestationRecord<M: ManagedTypeApi> {
    pub token_id: ManagedBuffer<M>,
    pub subject: ManagedAddress<M>,
    pub attestation_type: ManagedBuffer<M>,
    pub evidence_hash: ManagedBuffer<M>,
    pub approved: bool,
    pub attested_round: u64,
}

#[multiversx_sc::contract]
pub trait DrwaAttestation {
    #[init]
    fn init(&self, auditor: ManagedAddress) {
        self.auditor().set(auditor);
    }

    #[only_owner]
    #[endpoint(setAuditor)]
    fn set_auditor(&self, auditor: ManagedAddress) {
        require!(!auditor.is_zero(), "auditor must not be zero");
        self.pending_auditor().set(&auditor);
        self.drwa_auditor_proposed_event(&auditor);
    }

    #[endpoint(acceptAuditor)]
    fn accept_auditor(&self) {
        require!(!self.pending_auditor().is_empty(), "pending auditor not set");

        let caller = self.blockchain().get_caller();
        let pending = self.pending_auditor().get();
        require!(caller == pending, "caller not pending auditor");

        self.auditor().set(&pending);
        self.pending_auditor().clear();
        self.drwa_auditor_accepted_event(&pending);
    }

    #[endpoint(recordAttestation)]
    fn record_attestation(
        &self,
        token_id: ManagedBuffer,
        subject: ManagedAddress,
        attestation_type: ManagedBuffer,
        evidence_hash: ManagedBuffer,
        approved: bool,
    ) {
        let caller = self.blockchain().get_caller();
        require!(caller == self.auditor().get(), "caller not auditor");

        let record = AttestationRecord {
            token_id: token_id.clone(),
            subject: subject.clone(),
            attestation_type,
            evidence_hash,
            approved,
            attested_round: self.blockchain().get_block_round(),
        };

        self.attestation(&token_id, &subject).set(record);
    }

    #[view(getAttestation)]
    #[storage_mapper("attestation")]
    fn attestation(
        &self,
        token_id: &ManagedBuffer,
        subject: &ManagedAddress,
    ) -> SingleValueMapper<AttestationRecord<Self::Api>>;

    #[storage_mapper("auditor")]
    fn auditor(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("pendingAuditor")]
    fn pending_auditor(&self) -> SingleValueMapper<ManagedAddress>;

    #[event("drwaAuditorProposed")]
    fn drwa_auditor_proposed_event(&self, #[indexed] auditor: &ManagedAddress);

    #[event("drwaAuditorAccepted")]
    fn drwa_auditor_accepted_event(&self, #[indexed] auditor: &ManagedAddress);
}
