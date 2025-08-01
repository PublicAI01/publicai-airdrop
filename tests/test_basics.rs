use anyhow::Result;
use near_sdk::json_types::U128;
use near_sdk::Gas;
use near_workspaces::{compile_project, sandbox, types::NearToken, Account, Contract};
use serde_json::json;

/// Integration test that deploys the real airdrop & token contracts and shows that
/// claim_airdrop() creates a failing cross-contract call (ft_transfer on the user account).
#[tokio::test]
async fn test_claim_airdrop_cross_contract_failure() -> Result<()> {
    // 1. Spin up a local sandbox network
    let worker = sandbox().await?;

    // 2. Compile the token and airdrop contracts to WASM
    let token_wasm = compile_project("../publicai-token").await?;
    let airdrop_wasm = compile_project(".").await?; // current crate

    // 3. Deploy the token contract
    let token_contract: Contract = worker.dev_deploy(&token_wasm).await?;

    // Initialize token contract
    let root_account = worker.root_account()?;

    let metadata = json!({
        "spec": "ft-1.0.0",
        "name": "Test Token",
        "symbol": "TT",
        "decimals": 18,
        "icon": null,
        "reference": null,
        "reference_hash": null
    });

    let _ = root_account
        .call(token_contract.id(), "new")
        .args_json(json!({
            "owner_id": root_account.id(),
            "total_supply": U128(1_000_000_000u128),
            "metadata": metadata
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?; // Unwrap to catch init failure

    // 4. Deploy the airdrop contract
    let airdrop_contract: Contract = worker.dev_deploy(&airdrop_wasm).await?;

    let _ = root_account
        .call(airdrop_contract.id(), "new")
        .args_json((
            root_account.id(),
            token_contract.id(),
            "eef6e78d1a41f5778535f2f88c437a38ad2b693c13e1f8146de64687c5d7144a",
        ))
        .transact()
        .await?
        .into_result()?; // Unwrap to catch init failure

    // 5. Register airdrop_contract
    let _ = root_account
        .call(token_contract.id(), "storage_deposit")
        .args_json(json!({ "account_id": airdrop_contract.id(), "registration_only": null }))
        .deposit(NearToken::from_yoctonear(
            1_250_000_000_000_000_000_000_000u128,
        ))
        .transact()
        .await?
        .into_result()?; // Unwrap to catch failure

    // 6. Root calls claim_airdrop(),should failed because of airdrop_contract balance 0.
    let mut airdrop_exec = root_account
        .call(airdrop_contract.id(), "claim_airdrop")
        .args_json(json!({
            "amount": U128(400u128),
            "merkle_proof": ["154a0a614231d830d36a51e980c0cb836e8d2d718345e6c5e0e10bb3687ddb99"
                ,"eb41fc2783d2cb099b754cd5037b3229813581a1720ea692694af28d2db7e415"]
        }))
        .deposit(NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?;
    assert_eq!(airdrop_exec.is_success(), false, "Airdrop should failed");

    // Wait for the claim_airdrop cross-contract call to complete
    worker.fast_forward(10).await?;

    // 7.Transfer tokens to airdrop_contract
    let _ = root_account
        .call(token_contract.id(), "ft_transfer")
        .args_json(json!({
            "receiver_id": airdrop_contract.id(),
            "amount": U128(1_000_000_000u128),
            "memo": null
        }))
        .deposit(NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?
        .into_result()?; // Unwrap to catch failure

    worker.fast_forward(10).await?;

    let mut balance: U128 = root_account
        .view(token_contract.id(), "ft_balance_of")
        .args_json(json!({ "account_id": root_account.id() }))
        .await?
        .json()?;
    assert_eq!(balance.0, 0, "Transfer all balance");

    // 8. Verify root  receive tokens back
    airdrop_exec = root_account
        .call(airdrop_contract.id(), "claim_airdrop")
        .args_json(json!({
            "amount": U128(400u128),
            "merkle_proof": ["154a0a614231d830d36a51e980c0cb836e8d2d718345e6c5e0e10bb3687ddb99"
                ,"eb41fc2783d2cb099b754cd5037b3229813581a1720ea692694af28d2db7e415"]
        }))
        .deposit(NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?;
    assert_eq!(airdrop_exec.is_success(), true, "Airdrop should success");
    // Wait for the airdrop cross-contract call to complete
    worker.fast_forward(10).await?;

    balance = root_account
        .view(token_contract.id(), "ft_balance_of")
        .args_json(json!({ "account_id": root_account.id() }))
        .await?
        .json()?;
    assert_eq!(balance.0, 400, "Airdrop should success");
    // 9. Claim again,should faied.
    airdrop_exec = root_account
        .call(airdrop_contract.id(), "claim_airdrop")
        .args_json(json!({
            "amount": U128(400u128),
            "merkle_proof": ["154a0a614231d830d36a51e980c0cb836e8d2d718345e6c5e0e10bb3687ddb99"
                ,"eb41fc2783d2cb099b754cd5037b3229813581a1720ea692694af28d2db7e415"]
        }))
        .deposit(NearToken::from_yoctonear(1))
        .max_gas()
        .transact()
        .await?;
    assert_eq!(airdrop_exec.is_success(), false, "Airdrop should failed");
    Ok(())
}
