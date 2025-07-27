

---

# NEAR Airdrop Contract

This NEAR smart contract facilitates token airdrops for eligible users. It allows the owner to batch add airdrop recipients, update airdrop data, and enables users to claim their allocated tokens manually. The contract uses NEP-141 tokens as the airdrop asset.

---

## Features

1. **Batch Add Airdrops**:
    - The owner can batch add eligible user accounts and their respective token amounts.

2. **Update Airdrop Data**:
    - The owner can update the airdrop amount for specific users.

3. **Claim Airdrop**:
    - Eligible users can manually claim their tokens from the contract.

4. **Check Airdrop Eligibility**:
    - Users can query the contract to check if they are eligible for an airdrop and see the amount they can claim.

5. **NEP-141 Token Support**:
    - The contract uses NEP-141 tokens for the airdrop, and tokens are transferred using the `ft_transfer` method.

---

## Contract Details

### Constants

- **`owner_id`**: The account ID of the contract owner, who manages the airdrop data.
- **`token_contract`**: The NEP-141 token contract address that holds the tokens for the airdrop.
- **`airdrops`**: A mapping of user accounts to their allocated token amounts.

---

## Contract Methods

### **Initialization**

```rust
pub fn new(owner_id: AccountId, token_contract: AccountId) -> Self
```

Initializes the contract with the following parameters:
- `owner_id`: The account ID of the contract owner.
- `token_contract`: The NEP-141 token contract address.

#### Example Command:
```bash
near call <contract_account_id> new '{"owner_id": "<owner_account_id>", "token_contract": "<token_contract_id>"}' --accountId <owner_account_id>
```

---

### **Batch Add Airdrops**

```rust
pub fn add_airdrops(&mut self, recipients: Vec<AccountId>, amounts: Vec<U128>)
```

Allows the owner to batch add airdrop recipients and their respective token amounts.

#### Example Command:
```bash
near call <contract_account_id> add_airdrops '{"recipients": ["user1.testnet", "user2.testnet"], "amounts": ["100", "200"]}' --accountId <owner_account_id>
```

---

### **Update Airdrop**

```rust
pub fn update_airdrop(&mut self, recipient: AccountId, amount: U128)
```

Allows the owner to update the token amount for a specific recipient.

#### Example Command:
```bash
near call <contract_account_id> update_airdrop '{"recipient": "user1.testnet", "amount": "150"}' --accountId <owner_account_id>
```

---

### **Claim Airdrop**

```rust
pub fn claim_airdrop(&mut self)
```

Allows eligible users to claim their allocated tokens. After claiming, the user's entry is removed from the contract.

#### Example Command:
```bash
near call <contract_account_id> claim_airdrop '{}' --accountId <user_account_id>
```

---

### **Check Airdrop**

```rust
pub fn check_airdrop(&self, account_id: AccountId) -> U128
```

Allows users to check their eligibility for an airdrop and the amount they can claim.

#### Example Command:
```bash
near view <contract_account_id> check_airdrop '{"account_id": "user1.testnet"}'
```

---

### **Get Owner**

```rust
pub fn get_owner(&self) -> AccountId
```

Returns the account ID of the contract owner.

#### Example Command:
```bash
near view <contract_account_id> get_owner
```

---

## Deployment and Usage

### **Step 1: Compile the Contract**

Compile the contract to generate the `.wasm` file:
```bash
cargo build --target wasm32-unknown-unknown --release
```

### **Step 2: Deploy the Contract**

Deploy the contract to a NEAR account:
```bash
near deploy --accountId <contract_account_id> --wasmFile <path_to_wasm_file>
```

### **Step 3: Initialize the Contract**

Initialize the contract with the owner's account ID and NEP-141 token contract:
```bash
near call <contract_account_id> new '{"owner_id": "<owner_account_id>", "token_contract": "<token_contract_id>"}' --accountId <owner_account_id>
```

---

### Usage Commands

#### Add Airdrops:
```bash
near call <contract_account_id> add_airdrops '{"recipients": ["user1.testnet", "user2.testnet"], "amounts": ["100", "200"]}' --accountId <owner_account_id>
```

#### Update Airdrop:
```bash
near call <contract_account_id> update_airdrop '{"recipient": "user1.testnet", "amount": "150"}' --accountId <owner_account_id>
```

#### Claim Airdrop:
```bash
near call <contract_account_id> claim_airdrop '{}' --accountId <user_account_id>
```

#### Check Airdrop:
```bash
near view <contract_account_id> check_airdrop '{"account_id": "user1.testnet"}'
```

---

## Testing

### **Test Scenarios**

The provided tests cover the following scenarios:
1. **Contract Initialization**:
    - Ensures the contract initializes correctly with the specified `owner_id` and `token_contract`.

2. **Batch Add Airdrops**:
    - Tests that eligible users can be added and their amounts are correctly set.
    - Verifies that duplicate recipients are skipped.

3. **Update Airdrop**:
    - Tests updating airdrop amounts for specific recipients.
    - Ensures that updates to nonexistent recipients fail.

4. **Claim Airdrop**:
    - Tests that eligible users can claim tokens and their airdrop record is removed.
    - Verifies that users without airdrop eligibility cannot claim tokens.

5. **Check Airdrop**:
    - Confirms that users can query their claimable token amount.
    - Ensures that non-eligible users return `0`.

---

### **Run Tests**

Run the tests using the following command:
```bash
cargo test -- --nocapture
```

---

## Example Test Cases

### **Contract Initialization**

```rust
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
```

---

### **Batch Add Airdrops**

```rust
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
}
```

---

### **Claim Airdrop**

```rust
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
```

---

## Notes

- Ensure the contract owner has sufficient NEP-141 tokens to cover all airdrops.
- Users must manually claim their tokens using the `claim_airdrop` method.
- The contract owner can batch add recipients and update individual records.

---

## License

This contract is open-source and available under the MIT License.

---