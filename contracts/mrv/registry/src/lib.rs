#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use mrv_common::MrvReportProof;

#[multiversx_sc::contract]
pub trait MrvRegistry {
    #[init]
    fn init(&self) {}

    #[only_owner]
    #[endpoint(anchorReport)]
    fn anchor_report(
        &self,
        report_id: ManagedBuffer,
        public_tenant_id: ManagedBuffer,
        public_farm_id: ManagedBuffer,
        public_season_id: ManagedBuffer,
        report_hash: ManagedBuffer,
        hash_algo: ManagedBuffer,
        canonicalization: ManagedBuffer,
        methodology_version: u64,
        anchored_at: u64,
    ) {
        require!(!report_id.is_empty(), "empty report id");
        require!(!public_tenant_id.is_empty(), "empty public tenant id");
        require!(!public_farm_id.is_empty(), "empty public farm id");
        require!(!public_season_id.is_empty(), "empty public season id");
        require!(!report_hash.is_empty(), "empty report hash");
        require!(!hash_algo.is_empty(), "empty hash algo");
        require!(!canonicalization.is_empty(), "empty canonicalization");

        let proof = MrvReportProof {
            report_id: report_id.clone(),
            public_tenant_id: public_tenant_id.clone(),
            public_farm_id: public_farm_id.clone(),
            public_season_id: public_season_id.clone(),
            report_hash: report_hash.clone(),
            hash_algo: hash_algo.clone(),
            canonicalization: canonicalization.clone(),
            methodology_version,
            anchored_at,
        };

        if !self.report_proofs().contains_key(&report_id) {
            self.report_proofs().insert(report_id.clone(), proof.clone());
            self.proof_by_season().insert(
                (
                    public_tenant_id.clone(),
                    public_farm_id.clone(),
                    public_season_id.clone(),
                ),
                report_id.clone(),
            );
            self.mrv_report_anchored(
                &report_id,
                &public_tenant_id,
                &public_farm_id,
                &public_season_id,
                &report_hash,
                &hash_algo,
                &canonicalization,
                methodology_version,
                anchored_at,
            );

            return;
        }

        let existing = self
            .report_proofs()
            .get(&report_id)
            .unwrap_or_else(|| sc_panic!("missing proof"));
        require!(existing == proof, "conflicting report proof");
    }

    #[view(getReportProof)]
    fn get_report_proof(&self, report_id: ManagedBuffer) -> OptionalValue<MrvReportProof<Self::Api>> {
        match self.report_proofs().get(&report_id) {
            Some(proof) => OptionalValue::Some(proof),
            None => OptionalValue::None,
        }
    }

    #[view(getReportProofBySeason)]
    fn get_report_proof_by_season(
        &self,
        public_tenant_id: ManagedBuffer,
        public_farm_id: ManagedBuffer,
        public_season_id: ManagedBuffer,
    ) -> OptionalValue<MrvReportProof<Self::Api>> {
        let key = (public_tenant_id, public_farm_id, public_season_id);
        let report_id = match self.proof_by_season().get(&key) {
            Some(value) => value,
            None => return OptionalValue::None,
        };

        self.get_report_proof(report_id)
    }

    #[event("mrvReportAnchored")]
    fn mrv_report_anchored(
        &self,
        #[indexed] report_id: &ManagedBuffer,
        #[indexed] public_tenant_id: &ManagedBuffer,
        #[indexed] public_farm_id: &ManagedBuffer,
        #[indexed] public_season_id: &ManagedBuffer,
        report_hash: &ManagedBuffer,
        hash_algo: &ManagedBuffer,
        canonicalization: &ManagedBuffer,
        methodology_version: u64,
        anchored_at: u64,
    );

    #[storage_mapper("reportProofs")]
    fn report_proofs(&self) -> MapMapper<ManagedBuffer, MrvReportProof<Self::Api>>;

    #[storage_mapper("proofBySeason")]
    fn proof_by_season(
        &self,
    ) -> MapMapper<(ManagedBuffer, ManagedBuffer, ManagedBuffer), ManagedBuffer>;
}
