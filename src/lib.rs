use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, near, near_bindgen, AccountId, Gas, NearToken, PanicOnDefault, Promise};
use near_sdk::json_types::U128;

/// Contract to manage airdrops for eligible users.
#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct AirdropContract {
    // Owner of the contract
    owner_id: AccountId,
    // NEP-141 token contract address
    token_contract: AccountId,
    // Mapping of user accounts to their airdrop amounts
    airdrops: UnorderedMap<AccountId, u128>,
}

#[near]
impl AirdropContract {
    /// Initializes the contract with the given owner and NEP-141 token contract address.
    #[init]
    pub fn new(owner_id: AccountId, token_contract: AccountId) -> Self {
        assert!(!env::state_exists(), "The contract is already initialized.");
        Self {
            owner_id,
            token_contract,
            airdrops: UnorderedMap::new(b"a".to_vec()),
        }
    }

    /// Allows the owner to batch add airdrop recipients and their amounts.
    /// - `recipients`: A list of account IDs.
    /// - `amounts`: A list of token amounts corresponding to the recipients.
    pub fn add_airdrops(&mut self, recipients: Vec<AccountId>, amounts: Vec<U128>) {
        // Ensure only the owner can add airdrops
        assert_eq!(
            self.owner_id,
            env::predecessor_account_id(),
            "Only the owner can add airdrops."
        );

        // Ensure the lengths of the recipients and amounts match
        assert_eq!(
            recipients.len(),
            amounts.len(),
            "Recipients and amounts length mismatch."
        );

        // Add airdrops to the mapping
        for (i, recipient) in recipients.iter().enumerate() {
            let amount: u128 = amounts[i].0;

            // Check if the recipient already exists
            if self.airdrops.get(&recipient).is_some() {
                env::log_str(&format!(
                    "Account @{} already exists in the airdrop list. Skipping...",
                    recipient
                ));
                continue;
            }

            // Insert the new recipient and amount
            self.airdrops.insert(&recipient, &amount);
        }
    }

    /// Allows the owner to update the airdrop amount for a specific recipient.
    /// - `recipient`: The account ID of the recipient to update.
    /// - `amount`: The new airdrop amount for the recipient.
    pub fn update_airdrop(&mut self, recipient: AccountId, amount: U128) {
        // Ensure only the owner can update airdrops
        assert_eq!(
            self.owner_id,
            env::predecessor_account_id(),
            "Only the owner can update airdrops."
        );

        // Ensure the recipient exists in the airdrop list
        assert!(
            self.airdrops.get(&recipient).is_some(),
            "Account @{} is not in the airdrop list.",
            recipient
        );

        // Update the recipient's airdrop amount
        self.airdrops.insert(&recipient, &amount.0);
        env::log_str(&format!(
            "Airdrop for account @{} updated to {}.",
            recipient, amount.0
        ));
    }

    /// Allows users to claim their airdrop if they are eligible.
    pub fn claim_airdrop(&mut self) {
        let account_id = env::predecessor_account_id();
        let amount = self.airdrops.get(&account_id).unwrap_or(0);

        // Ensure the user has tokens to claim
        assert!(amount > 0, "You have no tokens to claim.");

        // Remove the user's entry from the airdrops mapping
        self.airdrops.remove(&account_id);

        // Transfer the tokens to the user using NEP-141's `ft_transfer`
        Promise::new(self.token_contract.clone()).function_call(
            "ft_transfer".to_string(),
            near_sdk::serde_json::json!({
                "receiver_id": account_id,
                "amount": U128(amount),
            })
                .to_string()
                .into_bytes(),
            NearToken::from_yoctonear(1), // Attach 1 yoctoNEAR for cross-contract call
            Gas::from_gas(50_000_000_000_000),
        );

        // Log the claim
        env::log_str(&format!(
            "Account @{} claimed {} tokens from @{}.",
            account_id, amount, self.token_contract
        ));
    }

    /// Allows users to check if they are eligible for an airdrop and the amount they can claim.
    /// Returns the amount of tokens the user can claim, or 0 if they are not eligible.
    pub fn check_airdrop(&self, account_id: AccountId) -> U128 {
        U128(self.airdrops.get(&account_id).unwrap_or(0))
    }

    /// Returns the owner of the contract.
    pub fn get_owner(&self) -> AccountId {
        self.owner_id.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::{testing_env, test_utils::VMContextBuilder, AccountId, Gas};
    use near_sdk::json_types::U128;

    // Constants for testing
    const TOKEN_CONTRACT: &str = "token.testnet";
    const OWNER: &str = "owner.testnet";
    const USER1: &str = "user1.testnet";
    const USER2: &str = "user2.testnet";
    const USER3: &str = "user3.testnet";

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
    fn test_initialize_contract() {
        let context = get_context(OWNER.parse::<AccountId>().unwrap(), 0);
        testing_env!(context.build());

        let contract = AirdropContract::new(
            OWNER.parse::<AccountId>().unwrap(),
            TOKEN_CONTRACT.parse::<AccountId>().unwrap(),
        );

        assert_eq!(
            contract.get_owner(),
            OWNER.parse::<AccountId>().unwrap()
        );
    }

    #[test]
    fn test_add_airdrops() {
        let context = get_context(OWNER.parse::<AccountId>().unwrap(), 0);
        testing_env!(context.build());

        let mut contract = AirdropContract::new(
            OWNER.parse::<AccountId>().unwrap(),
            TOKEN_CONTRACT.parse::<AccountId>().unwrap(),
        );

        let recipients = vec![
            USER1.parse::<AccountId>().unwrap(),
            USER2.parse::<AccountId>().unwrap(),
        ];
        let amounts = vec![U128(100), U128(200)];

        contract.add_airdrops(recipients.clone(), amounts.clone());

        assert_eq!(
            contract.check_airdrop(USER1.parse::<AccountId>().unwrap()).0,
            100
        );
        assert_eq!(
            contract.check_airdrop(USER2.parse::<AccountId>().unwrap()).0,
            200
        );
        assert_eq!(
            contract.check_airdrop(USER3.parse::<AccountId>().unwrap()).0,
            0
        );
    }

    #[test]
    #[should_panic(expected = "Only the owner can add airdrops.")]
    fn test_add_airdrops_not_owner() {
        let context = get_context(USER1.parse::<AccountId>().unwrap(), 0);
        testing_env!(context.build());

        let mut contract = AirdropContract::new(
            OWNER.parse::<AccountId>().unwrap(),
            TOKEN_CONTRACT.parse::<AccountId>().unwrap(),
        );

        let recipients = vec![USER1.parse::<AccountId>().unwrap()];
        let amounts = vec![U128(100)];

        contract.add_airdrops(recipients, amounts);
    }

    #[test]
    fn test_add_airdrops_with_duplicates() {
        let context = get_context(OWNER.parse::<AccountId>().unwrap(), 0);
        testing_env!(context.build());

        let mut contract = AirdropContract::new(
            OWNER.parse::<AccountId>().unwrap(),
            TOKEN_CONTRACT.parse::<AccountId>().unwrap(),
        );

        let recipients = vec![
            USER1.parse::<AccountId>().unwrap(),
            USER2.parse::<AccountId>().unwrap(),
        ];
        let amounts = vec![U128(100), U128(200)];

        // Add recipients for the first time
        contract.add_airdrops(recipients.clone(), amounts.clone());

        // Add the same recipients again (should skip duplicates)
        contract.add_airdrops(recipients.clone(), amounts.clone());

        // Verify that the amounts are unchanged
        assert_eq!(
            contract.check_airdrop(USER1.parse::<AccountId>().unwrap()).0,
            100
        );
        assert_eq!(
            contract.check_airdrop(USER2.parse::<AccountId>().unwrap()).0,
            200
        );
    }

    #[test]
    fn test_update_airdrop() {
        let context = get_context(OWNER.parse::<AccountId>().unwrap(), 0);
        testing_env!(context.build());

        let mut contract = AirdropContract::new(
            OWNER.parse::<AccountId>().unwrap(),
            TOKEN_CONTRACT.parse::<AccountId>().unwrap(),
        );

        let recipients = vec![USER1.parse::<AccountId>().unwrap()];
        let amounts = vec![U128(100)];
        contract.add_airdrops(recipients.clone(), amounts.clone());

        // Update the recipient's airdrop amount
        contract.update_airdrop(USER1.parse::<AccountId>().unwrap(), U128(150));

        // Verify the updated amount
        assert_eq!(
            contract.check_airdrop(USER1.parse::<AccountId>().unwrap()).0,
            150
        );
    }

    #[test]
    #[should_panic(expected = "Account @user3.testnet is not in the airdrop list.")]
    fn test_update_airdrop_non_existent() {
        let context = get_context(OWNER.parse::<AccountId>().unwrap(), 0);
        testing_env!(context.build());

        let mut contract = AirdropContract::new(
            OWNER.parse::<AccountId>().unwrap(),
            TOKEN_CONTRACT.parse::<AccountId>().unwrap(),
        );

        // Attempt to update a non-existent recipient
        contract.update_airdrop(USER3.parse::<AccountId>().unwrap(), U128(150));
    }

    #[test]
    fn test_claim_airdrop() {
        let context = get_context(OWNER.parse::<AccountId>().unwrap(), 0);
        testing_env!(context.build());

        let mut contract = AirdropContract::new(
            OWNER.parse::<AccountId>().unwrap(),
            TOKEN_CONTRACT.parse::<AccountId>().unwrap(),
        );

        let recipients = vec![USER1.parse::<AccountId>().unwrap()];
        let amounts = vec![U128(100)];
        contract.add_airdrops(recipients.clone(), amounts.clone());

        // Simulate the user claiming the airdrop
        let context = get_context(USER1.parse::<AccountId>().unwrap(), 0);
        testing_env!(context.build());

        contract.claim_airdrop();

        // Verify that the user has no more tokens to claim
        assert_eq!(
            contract.check_airdrop(USER1.parse::<AccountId>().unwrap()).0,
            0
        );
    }

    #[test]
    #[should_panic(expected = "You have no tokens to claim.")]
    fn test_claim_airdrop_no_tokens() {
        let context = get_context(USER2.parse::<AccountId>().unwrap(), 0);
        testing_env!(context.build());

        let mut contract = AirdropContract::new(
            OWNER.parse::<AccountId>().unwrap(),
            TOKEN_CONTRACT.parse::<AccountId>().unwrap(),
        );

        // Attempt to claim airdrop without being eligible
        contract.claim_airdrop();
    }
}


