
---

# Merkle Tree Airdrop NEAR Contract

This smart contract enables efficient and scalable airdrop distributions on the NEAR blockchain by using a Merkle Tree to verify user eligibility and claimed amounts. It supports NEP-141 tokens and ensures users can only claim once.

## Features

- **Merkle Tree-based Airdrop**: Store airdrop eligibility off-chain and use a Merkle root on-chain for verification.
- **Efficient Claiming**: Users provide a Merkle proof and amount to claim; the contract checks eligibility and prevents double claims.
- **Support for NEP-141 Tokens**: Sends any NEP-141 token as the airdrop asset.
- **Owner Controlled**: Only the contract owner can update the Merkle root.

## Contract Methods

### Initialization

```rust
pub fn new(owner_id: AccountId, token_contract: AccountId, merkle_root: String) -> Self
```
Initializes the contract with the owner, the NEP-141 token contract, and the initial Merkle root.

### Update Merkle Root

```rust
pub fn update_merkle_root(&mut self, merkle_root: String)
```
Updates the Merkle root (only callable by the owner).

### Claim Airdrop

```rust
pub fn claim_airdrop(&mut self, amount: U128, merkle_proof: Vec<String>)
```
Allows eligible users to claim their airdrop by providing the intended claim amount and a valid Merkle proof for `(account_id, amount)`.

### Verify Merkle Proof

```rust
pub fn verify_merkle_proof(leaf: String, root: &String, proof: &Vec<String>) -> bool
```
Utility function to verify a Merkle proof for a given leaf and root.

### Get Merkle Root

```rust
pub fn get_merkle_root(&self) -> String
```
Returns the current Merkle root used for airdrop verification.

### Check if Claimed

```rust
pub fn has_claimed(&self, account_id: AccountId) -> bool
```
Returns `true` if the account has already claimed the airdrop.

## Usage

1. **Generate the Merkle Tree**: Off-chain, use your airdrop list to generate Merkle leaves (e.g., `account_id + amount`), and compute the Merkle root and proofs for each user.
2. **Deploy and Initialize**: Deploy the contract to NEAR, then initialize it with the Merkle root and token contract.
3. **Distribute Proofs**: Provide users with their claim amount and Merkle proof.
4. **Claim**: Users call `claim_airdrop(amount, proof)` with their account, amount, and proof. The contract checks the proof, prevents double claims, and sends NEP-141 tokens to the user.

## Security

- Only eligible users (with a valid proof) can claim.
- Every account can claim only once.
- Only the owner can update the airdrop Merkle root.

## Example

```rust
AirdropContract::new(
    "owner.near".parse().unwrap(),
    "token.near".parse().unwrap(),
    "merkle_root_as_hex_string".to_string(),
);
```

## License

MIT

---

**Note:**
- Generating Merkle roots and proofs is done off-chain (e.g., using JavaScript libraries like `merkletreejs`).
- The contract expects the proof as a Vec of hex-encoded hashes.
- The leaf node format is typically `account_id + amount` (as a string) for hashing.
- For production, always audit the code and test thoroughly.