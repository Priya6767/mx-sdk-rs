#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

const DEFAULT_IDENTITY_VALIDITY_ROUNDS: u64 = 10_000;

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem, Clone)]
pub struct IdentityRecord<M: ManagedTypeApi> {
    pub legal_name: ManagedBuffer<M>,
    pub jurisdiction_code: ManagedBuffer<M>,
    pub registration_number: ManagedBuffer<M>,
    pub entity_type: ManagedBuffer<M>,
    pub kyc_status: ManagedBuffer<M>,
    pub aml_status: ManagedBuffer<M>,
    pub investor_class: ManagedBuffer<M>,
    pub expiry_round: u64,
}

#[multiversx_sc::contract]
pub trait DrwaIdentityRegistry {
    #[init]
    fn init(&self, governance: ManagedAddress) {
        self.governance().set(governance);
    }

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

    #[endpoint(registerIdentity)]
    fn register_identity(
        &self,
        subject: ManagedAddress,
        legal_name: ManagedBuffer,
        jurisdiction_code: ManagedBuffer,
        registration_number: ManagedBuffer,
        entity_type: ManagedBuffer,
    ) {
        self.require_governance_or_owner();

        let record = IdentityRecord {
            legal_name,
            jurisdiction_code,
            registration_number,
            entity_type,
            kyc_status: ManagedBuffer::from(b"pending"),
            aml_status: ManagedBuffer::from(b"pending"),
            investor_class: ManagedBuffer::new(),
            expiry_round: self
                .blockchain()
                .get_block_round()
                .saturating_add(DEFAULT_IDENTITY_VALIDITY_ROUNDS),
        };

        self.identity(&subject).set(record);
    }

    #[endpoint(updateComplianceStatus)]
    fn update_compliance_status(
        &self,
        subject: ManagedAddress,
        kyc_status: ManagedBuffer,
        aml_status: ManagedBuffer,
        investor_class: ManagedBuffer,
        expiry_round: u64,
    ) {
        self.require_governance_or_owner();

        self.identity(&subject).update(|record| {
            record.kyc_status = kyc_status;
            record.aml_status = aml_status;
            record.investor_class = investor_class;
            record.expiry_round = expiry_round;
        });
    }

    #[view(getIdentity)]
    #[storage_mapper("identity")]
    fn identity(&self, subject: &ManagedAddress) -> SingleValueMapper<IdentityRecord<Self::Api>>;

    #[view(getGovernance)]
    #[storage_mapper("governance")]
    fn governance(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getPendingGovernance)]
    #[storage_mapper("pendingGovernance")]
    fn pending_governance(&self) -> SingleValueMapper<ManagedAddress>;

    #[event("drwaGovernanceProposed")]
    fn drwa_governance_proposed_event(&self, #[indexed] governance: &ManagedAddress);

    #[event("drwaGovernanceAccepted")]
    fn drwa_governance_accepted_event(&self, #[indexed] governance: &ManagedAddress);

    fn require_governance_or_owner(&self) {
        let caller = self.blockchain().get_caller();
        let governance = self.governance().get();
        require!(
            caller == governance || caller == self.blockchain().get_owner_address(),
            "caller not authorized"
        );
    }
}
