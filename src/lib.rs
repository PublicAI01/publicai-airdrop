use near_sdk::json_types::U128;
use near_sdk::{
    assert_one_yocto, env, log, near, require, serde_json, AccountId, Gas, NearToken,
    PanicOnDefault, Promise,
};

/// Contract to manage airdrops using a Merkle Tree
#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct AirdropContract {
    // Owner of the contract
    owner_id: AccountId,
    // NEP-141 token contract address
    token_contract: AccountId,
    // Root hash of the Merkle tree
    merkle_root: String,
    // Mapping to keep track of claimed accounts
    claimed: std::collections::HashSet<AccountId>,
}

#[near]
impl AirdropContract {
    /// Initializes the contract with the given owner and NEP-141 token contract address.
    #[init]
    pub fn new(owner_id: AccountId, token_contract: AccountId, merkle_root: String) -> Self {
        assert!(!env::state_exists(), "The contract is already initialized.");
        Self {
            owner_id,
            token_contract,
            merkle_root,
            claimed: std::collections::HashSet::new(),
        }
    }

    /// Updates the Merkle root (only callable by the owner).
    /// - `merkle_root`: The new Merkle root representing the airdrop list.
    #[payable]
    pub fn update_merkle_root(&mut self, merkle_root: String) {
        assert_one_yocto();
        assert_eq!(
            self.owner_id,
            env::predecessor_account_id(),
            "Only the owner can update the Merkle root."
        );
        self.merkle_root = merkle_root;
        env::log_str(&format!("Merkle root updated to {}", self.merkle_root));
    }

    #[payable]
    pub fn update_owner(&mut self, new_owner: AccountId) -> bool {
        assert_one_yocto();
        require!(
            env::predecessor_account_id() == self.owner_id,
            "Owner's method"
        );
        require!(!new_owner.as_str().is_empty(), "New owner cannot be empty");
        log!("Owner updated from {} to {}", self.owner_id, new_owner);
        self.owner_id = new_owner;
        true
    }

    /// Allows users to claim their airdrop if they are eligible.
    /// - `amount`: The amount of tokens the user claims.
    /// - `merkle_proof`: The Merkle proof validating the user's claim.
    #[payable]
    pub fn claim_airdrop(&mut self, amount: U128, merkle_proof: Vec<String>) -> Promise {
        assert_one_yocto();
        let account_id = env::predecessor_account_id();

        // Ensure the user has not already claimed
        assert!(
            !self.claimed.contains(&account_id),
            "You have already claimed your airdrop."
        );

        // Verify the Merkle proof
        let leaf = format!("{}:{}", account_id, amount.0);
        assert!(
            Self::verify_merkle_proof(leaf, &self.merkle_root, &merkle_proof),
            "Merkle proof verification failed."
        );

        // Mark the account as claimed
        self.claimed.insert(account_id.clone());

        // Always call storage_deposit first, regardless of registration status
        Promise::new(self.token_contract.clone())
            .function_call(
                "storage_deposit".to_string(),
                near_sdk::serde_json::json!({
                    "account_id": account_id,
                    "registration_only": true
                })
                .to_string()
                .into_bytes(),
                NearToken::from_yoctonear(1_250_000_000_000_000_000_000),
                Gas::from_gas(10_000_000_000_000),
            )
            // Chain to transfer tokens after storage_deposit
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_gas(40_000_000_000_000))
                    .on_storage_deposit_then_transfer(account_id, amount),
            )
    }

    /// Callback: After storage_deposit, attempt to transfer the airdrop tokens.
    #[private]
    pub fn on_storage_deposit_then_transfer(
        &mut self,
        account_id: AccountId,
        amount: U128,
        #[callback_result] call_result: Result<Option<serde_json::Value>, near_sdk::PromiseError>,
    ) -> Promise {
        // If storage_deposit failed, revert and do not transfer tokens
        if call_result.is_err() {
            self.claimed.remove(&account_id);
            return Promise::new(env::current_account_id());
        }
        Promise::new(self.token_contract.clone())
            .function_call(
                "ft_transfer".to_string(),
                serde_json::json!({
                    "receiver_id": account_id.clone(),
                    "amount": amount,
                })
                .to_string()
                .into_bytes(),
                NearToken::from_yoctonear(1),
                Gas::from_gas(20_000_000_000_000),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_gas(5_000_000_000_000))
                    .on_ft_transfer_then_claimed(account_id, amount),
            )
    }

    /// Callback: After ft_transfer, only then mark the account as claimed.
    #[private]
    pub fn on_ft_transfer_then_claimed(
        &mut self,
        account_id: AccountId,
        amount: U128,
        #[callback_result] call_result: Result<(), near_sdk::PromiseError>,
    ) -> bool {
        if call_result.is_err() {
            self.claimed.remove(&account_id);
            return false;
        }
        env::log_str(&format!(
            "Account @{} claimed {} tokens from @{}.",
            account_id, amount.0, self.token_contract
        ));
        true
    }

    /// Verifies a Merkle proof.
    /// - `leaf`: The leaf node (e.g., "account_id + amount").
    /// - `root`: The root of the Merkle tree.
    /// - `proof`: The Merkle proof (an array of sibling hashes).
    /// Returns `true` if the proof is valid, `false` otherwise.
    pub fn verify_merkle_proof(leaf: String, root: &String, proof: &Vec<String>) -> bool {
        let mut hash = env::keccak256(leaf.as_bytes());
        for sibling in proof {
            let sibling_hash = hex::decode(sibling).expect("Invalid hex in Merkle proof.");
            if hash < sibling_hash {
                hash = env::keccak256(&[hash.as_slice(), sibling_hash.as_slice()].concat());
            } else {
                hash = env::keccak256(&[sibling_hash.as_slice(), hash.as_slice()].concat());
            }
        }
        hex::encode(hash) == *root
    }

    /// Returns the current Merkle root.
    pub fn get_merkle_root(&self) -> String {
        self.merkle_root.clone()
    }

    /// Checks if an account has already claimed their airdrop.
    pub fn has_claimed(&self, account_id: AccountId) -> bool {
        self.claimed.contains(&account_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::json_types::U128;
    use near_sdk::{test_utils::VMContextBuilder, testing_env, AccountId, Gas};

    // Constants for testing
    const TOKEN_CONTRACT: &str = "token.testnet";
    const OWNER: &str = "owner.testnet";
    const USER1: &str = "user1.testnet";

    /// Helper function to create a mock context.
    fn get_context(predecessor: AccountId, attached_deposit: u128) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .predecessor_account_id(predecessor)
            .attached_deposit(NearToken::from_yoctonear(attached_deposit))
            .prepaid_gas(Gas::from_gas(300_000_000_000_000)); // Allocate sufficient gas for testing
        builder
    }
    #[test]
    fn test_merkle_proof_verification() {
        let context = get_context(OWNER.parse::<AccountId>().unwrap(), 0);
        testing_env!(context.build());

        AirdropContract::new(
            OWNER.parse::<AccountId>().unwrap(),
            TOKEN_CONTRACT.parse::<AccountId>().unwrap(),
            "af6df487c9daa2c7d6ec7fb9a33f22d6af13323c1f0d9b1a7df3ec0aaea02e94".to_string(), // Replace with real Merkle Root
        );

        // Example Merkle proof for "user1.testnet + : + 100"
        let leaf = "user1.testnet:100".to_string();
        let proof = vec![
            "154a0a614231d830d36a51e980c0cb836e8d2d718345e6c5e0e10bb3687ddb99".to_string(),
            "86b99e84ab1b07c73445edf731d9c0d876c6229e36a5bf22c210690e2cdc18b2".to_string(),
        ];

        let valid = AirdropContract::verify_merkle_proof(
            leaf,
            &"af6df487c9daa2c7d6ec7fb9a33f22d6af13323c1f0d9b1a7df3ec0aaea02e94".to_string(),
            &proof,
        );

        assert!(valid, "Merkle proof should be valid for user1.testnet.");
    }

    #[test]
    #[should_panic]
    fn test_claim_airdrop() {
        let context = get_context(OWNER.parse::<AccountId>().unwrap(), 0);
        testing_env!(context.build());

        let mut contract = AirdropContract::new(
            OWNER.parse::<AccountId>().unwrap(),
            TOKEN_CONTRACT.parse::<AccountId>().unwrap(),
            "42bb039d55571a5564e772449aab51904f292f69ea5efb6becde8f8f5c37d643".to_string(), // Replace with real Merkle Root
        );

        // Example Merkle proof for "user1.testnet + : + 100"
        let proof = vec![
            "bcd3ddbb88881cf79a7f4de2b1024a50f83356856ff31367ecac4526172106a4".to_string(),
            "9674039b49ffcb659ac14ed833f9e6c9070d457a36ef0a5a28bc257e145c8160".to_string(),
        ];

        let context = get_context(USER1.parse::<AccountId>().unwrap(), 1);
        testing_env!(context.build());

        contract.claim_airdrop(U128(100), proof);

        // Verify that the user cannot claim again
        let context = get_context(USER1.parse::<AccountId>().unwrap(), 1);
        testing_env!(context.build());

        contract.claim_airdrop(U128(100), vec![]);
    }
}
