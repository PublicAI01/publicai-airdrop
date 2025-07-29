use near_sdk::json_types::U128;
use near_sdk::{env, near, serde_json, AccountId, Gas, NearToken, PanicOnDefault, Promise};

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
    pub fn update_merkle_root(&mut self, merkle_root: String) {
        assert_eq!(
            self.owner_id,
            env::predecessor_account_id(),
            "Only the owner can update the Merkle root."
        );
        self.merkle_root = merkle_root;
        env::log_str(&format!("Merkle root updated to {}", self.merkle_root));
    }

    /// Allows users to claim their airdrop if they are eligible.
    /// - `amount`: The amount of tokens the user claims.
    /// - `merkle_proof`: The Merkle proof validating the user's claim.
    pub fn claim_airdrop(&mut self, amount: U128, merkle_proof: Vec<String>) -> Promise {
        let account_id = env::predecessor_account_id();

        // Ensure the user has not already claimed
        assert!(
            !self.claimed.contains(&account_id),
            "You have already claimed your airdrop."
        );

        // Verify the Merkle proof
        let leaf = format!("{}{}", account_id, amount.0);
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
        #[callback_result] _call_result: Result<Option<serde_json::Value>, near_sdk::PromiseError>,
    ) -> Promise {
        Promise::new(self.token_contract.clone()).function_call(
            "ft_transfer".to_string(),
            serde_json::json!({
                "receiver_id": account_id,
                "amount": amount,
            })
            .to_string()
            .into_bytes(),
            NearToken::from_yoctonear(1),
            Gas::from_gas(20_000_000_000_000),
        )
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
            "6867b92646b924dee3f594488111e91ced74800da03a4be76e8175277c380f4d".to_string(), // Replace with real Merkle Root
        );

        // Example Merkle proof for "user1.testnet + 100"
        let leaf = "user1.testnet100".to_string();
        let proof = vec![
            "ed356db93bc0a636f60329c5e37b36bc9b6f5e5ad422438b051cbb66aa44603b".to_string(),
            "bff38e5fe57bd24169c460f1f151a39414a78be908898331dbfc5fa76e81a0c8".to_string(),
        ];

        let valid = AirdropContract::verify_merkle_proof(
            leaf,
            &"6867b92646b924dee3f594488111e91ced74800da03a4be76e8175277c380f4d".to_string(),
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
            "4128b84507d5c7c726e884b7c26c6db7c9b1f5245855dcd31474c8d6be5ca625".to_string(), // Replace with real Merkle Root
        );

        // Example Merkle proof for "user1.testnet + 100"
        let proof = vec![
            "bff38e5fe57bd24169c460f1f151a39414a78be908898331dbfc5fa76e81a0c8".to_string(),
            "ed356db93bc0a636f60329c5e37b36bc9b6f5e5ad422438b051cbb66aa44603b".to_string(),
        ];

        let context = get_context(USER1.parse::<AccountId>().unwrap(), 0);
        testing_env!(context.build());

        contract.claim_airdrop(U128(100), proof);

        // Verify that the user cannot claim again
        let context = get_context(USER1.parse::<AccountId>().unwrap(), 0);
        testing_env!(context.build());

        contract.claim_airdrop(U128(100), vec![]);
    }
}
