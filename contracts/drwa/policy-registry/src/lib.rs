#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use multiversx_sc::api::HandleConstraints;

use drwa_common::{
    build_sync_hook_payload, DrwaCallerDomain, DrwaSyncEnvelope, DrwaSyncOperation,
    DrwaSyncOperationType, DrwaTokenPolicy, serialize_sync_envelope_payload,
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

#[multiversx_sc::contract]
pub trait DrwaPolicyRegistry {
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

    #[endpoint(setTokenPolicy)]
    fn set_token_policy(
        &self,
        token_id: ManagedBuffer,
        drwa_enabled: bool,
        global_pause: bool,
        strict_auditor_mode: bool,
        metadata_protection_enabled: bool,
        allowed_investor_classes: ManagedVec<ManagedBuffer>,
        allowed_jurisdictions: ManagedVec<ManagedBuffer>,
    ) -> DrwaSyncEnvelope<Self::Api> {
        self.require_governance_or_owner();

        let next_version = self.token_policy_version(&token_id).get() + 1;

        let policy = DrwaTokenPolicy {
            drwa_enabled,
            global_pause,
            strict_auditor_mode,
            metadata_protection_enabled,
            token_policy_version: next_version,
            allowed_investor_classes,
            allowed_jurisdictions,
        };

        self.token_policy(&token_id).set(policy.clone());
        self.token_policy_version(&token_id).set(next_version);
        self.drwa_token_policy_event(
            &token_id,
            policy.drwa_enabled,
            policy.global_pause,
            policy.strict_auditor_mode,
            next_version,
        );

        let body = self.serialize_policy_json(&policy);
        let mut operations = ManagedVec::new();
        operations.push(DrwaSyncOperation {
            operation_type: DrwaSyncOperationType::TokenPolicy,
            token_id: token_id.clone(),
            holder: ManagedAddress::default(),
            version: next_version,
            body,
        });

        let caller_domain = DrwaCallerDomain::PolicyRegistry;
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

    #[view(getTokenPolicy)]
    #[storage_mapper("tokenPolicy")]
    fn token_policy(
        &self,
        token_id: &ManagedBuffer,
    ) -> SingleValueMapper<DrwaTokenPolicy<Self::Api>>;

    #[view(getTokenPolicyVersion)]
    #[storage_mapper("tokenPolicyVersion")]
    fn token_policy_version(&self, token_id: &ManagedBuffer) -> SingleValueMapper<u64>;

    #[view(getGovernance)]
    #[storage_mapper("governance")]
    fn governance(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getPendingGovernance)]
    #[storage_mapper("pendingGovernance")]
    fn pending_governance(&self) -> SingleValueMapper<ManagedAddress>;

    #[event("drwaTokenPolicy")]
    fn drwa_token_policy_event(
        &self,
        #[indexed] token_id: &ManagedBuffer,
        #[indexed] drwa_enabled: bool,
        #[indexed] global_pause: bool,
        #[indexed] strict_auditor_mode: bool,
        #[indexed] token_policy_version: u64,
    );

    #[event("drwaGovernanceProposed")]
    fn drwa_governance_proposed_event(&self, #[indexed] governance: &ManagedAddress);

    #[event("drwaGovernanceAccepted")]
    fn drwa_governance_accepted_event(&self, #[indexed] governance: &ManagedAddress);

    /// Serializes a token policy as a JSON object whose field names match the
    /// drwaTokenPolicyView JSON tags used by the Go enforcement decoder.
    /// Switching from the previous 12-byte binary format ensures that
    /// allowed_investor_classes and allowed_jurisdictions are included, fixing C-01.
    /// Validates that a policy key value (investor class or jurisdiction code)
    /// contains no characters that would break the hand-built JSON serialization.
    /// A corrupt JSON body that starts with '{' is NOT silently re-routed to the
    /// binary decoder on the Go side — it surfaces as a parse error and the
    /// enforcement gate falls back to deny-by-parse-error.
    ///
    /// Rejected bytes:
    ///   - anything outside the conservative identifier alphabet
    ///     [A-Za-z0-9._-]
    ///
    /// This contract intentionally avoids a general-purpose JSON string
    /// serializer in `no_std` and instead constrains policy keys to a
    /// safe identifier alphabet before hand-building the JSON body.
    fn require_json_safe_key(&self, key: &ManagedBuffer) {
        require!(!key.is_empty(), "policy key must not be empty");
        let len = key.len();
        for i in 0..len {
            let mut byte_buf = [0u8; 1];
            key.load_slice(i, &mut byte_buf);
            let b = byte_buf[0];
            let is_ascii_alpha = b.is_ascii_alphabetic();
            let is_ascii_digit = b.is_ascii_digit();
            let is_safe_punct = b == b'.' || b == b'_' || b == b'-';
            require!(
                is_ascii_alpha || is_ascii_digit || is_safe_punct,
                "policy key contains unsupported character"
            );
        }
    }

    fn serialize_policy_json(&self, policy: &DrwaTokenPolicy<Self::Api>) -> ManagedBuffer {
        for class in policy.allowed_investor_classes.iter() {
            self.require_json_safe_key(&class);
        }
        for jur in policy.allowed_jurisdictions.iter() {
            self.require_json_safe_key(&jur);
        }

        let mut body = ManagedBuffer::new();
        body.append_bytes(b"{\"drwa_enabled\":");
        body.append_bytes(if policy.drwa_enabled { b"true" } else { b"false" });
        body.append_bytes(b",\"global_pause\":");
        body.append_bytes(if policy.global_pause { b"true" } else { b"false" });
        body.append_bytes(b",\"strict_auditor_mode\":");
        body.append_bytes(if policy.strict_auditor_mode { b"true" } else { b"false" });
        body.append_bytes(b",\"metadata_protection_enabled\":");
        body.append_bytes(if policy.metadata_protection_enabled {
            b"true"
        } else {
            b"false"
        });
        if !policy.allowed_investor_classes.is_empty() {
            body.append_bytes(b",\"allowed_investor_classes\":{");
            let mut first = true;
            for class in policy.allowed_investor_classes.iter() {
                if !first {
                    body.append_bytes(b",");
                }
                body.append_bytes(b"\"");
                body.append(&class);
                body.append_bytes(b"\":true");
                first = false;
            }
            body.append_bytes(b"}");
        }
        if !policy.allowed_jurisdictions.is_empty() {
            body.append_bytes(b",\"allowed_jurisdictions\":{");
            let mut first = true;
            for jur in policy.allowed_jurisdictions.iter() {
                if !first {
                    body.append_bytes(b",");
                }
                body.append_bytes(b"\"");
                body.append(&jur);
                body.append_bytes(b"\":true");
                first = false;
            }
            body.append_bytes(b"}");
        }
        body.append_bytes(b"}");
        body
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
