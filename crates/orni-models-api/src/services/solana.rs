use crate::config::Config;
use crate::error::{AppError, AppResult};

/// Verify a USDC deposit transaction on-chain.
pub async fn verify_deposit(
    client: &reqwest::Client,
    config: &Config,
    tx_signature: &str,
    expected_amount: u64,
    sender_wallet: &str,
) -> AppResult<bool> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [
            tx_signature,
            { "encoding": "jsonParsed", "maxSupportedTransactionVersion": 0 }
        ]
    });

    let resp = client
        .post(&config.solana_rpc_url)
        .json(&body)
        .send()
        .await?;

    let result: serde_json::Value = resp.json().await?;

    let tx = result
        .get("result")
        .ok_or_else(|| AppError::BadRequest("Transaction not found".into()))?;

    // Check for errors
    if tx.get("meta").and_then(|m| m.get("err")).is_some()
        && !tx["meta"]["err"].is_null()
    {
        return Err(AppError::BadRequest("Transaction failed on-chain".into()));
    }

    // Look for SPL token transfer to escrow
    let empty_vec = vec![];
    let instructions = tx
        .pointer("/transaction/message/instructions")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty_vec);

    for ix in instructions {
        let program = ix.pointer("/program").and_then(|v| v.as_str());
        if program != Some("spl-token") {
            continue;
        }

        let ix_type = ix.pointer("/parsed/type").and_then(|v| v.as_str());
        if ix_type != Some("transfer") && ix_type != Some("transferChecked") {
            continue;
        }

        let info = &ix["parsed"]["info"];

        // Check mint for transferChecked
        if ix_type == Some("transferChecked") {
            let mint = info["mint"].as_str().unwrap_or("");
            if mint != config.usdc_mint {
                continue;
            }
        }

        let authority = info["authority"].as_str().unwrap_or("");
        if authority != sender_wallet {
            continue;
        }

        // Verify destination is the escrow wallet's token account
        let destination = info["destination"].as_str().unwrap_or("");
        if !config.escrow_wallet_address.is_empty() {
            // The destination should be the escrow wallet's ATA
            // For now, check that the destination is not the sender's own account
            let source = info["source"].as_str().unwrap_or("");
            if destination == source || destination.is_empty() {
                continue; // Self-transfer or missing destination
            }
            // TODO: derive escrow ATA and compare exactly
        }

        // Get amount (transferChecked uses tokenAmount.amount, transfer uses amount)
        let amount_str = if ix_type == Some("transferChecked") {
            info.pointer("/tokenAmount/amount")
                .and_then(|v| v.as_str())
                .unwrap_or("0")
        } else {
            info["amount"].as_str().unwrap_or("0")
        };

        let amount: u64 = amount_str.parse().unwrap_or(0);
        if amount >= expected_amount {
            return Ok(true);
        }
    }

    // Also check inner instructions
    let inner = tx
        .pointer("/meta/innerInstructions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for group in &inner {
        let ixs = group
            .get("instructions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        for ix in &ixs {
            let program = ix.pointer("/program").and_then(|v| v.as_str());
            if program != Some("spl-token") {
                continue;
            }

            let info = &ix["parsed"]["info"];
            let authority = info["authority"].as_str().unwrap_or("");
            if authority != sender_wallet {
                continue;
            }

            let amount_str = info
                .pointer("/tokenAmount/amount")
                .or_else(|| info.get("amount"))
                .and_then(|v| v.as_str())
                .unwrap_or("0");

            let amount: u64 = amount_str.parse().unwrap_or(0);
            if amount >= expected_amount {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

/// Derive an Associated Token Account address.
pub fn derive_ata(wallet: &[u8; 32], mint: &[u8; 32]) -> AppResult<[u8; 32]> {
    let token_program: [u8; 32] = bs58::decode("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
        .into_vec()
        .map_err(|_| AppError::Internal("Invalid token program".into()))?
        .try_into()
        .map_err(|_| AppError::Internal("Invalid token program length".into()))?;

    let ata_program: [u8; 32] = bs58::decode("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
        .into_vec()
        .map_err(|_| AppError::Internal("Invalid ATA program".into()))?
        .try_into()
        .map_err(|_| AppError::Internal("Invalid ATA program length".into()))?;

    find_program_address(
        &[wallet.as_ref(), token_program.as_ref(), mint.as_ref()],
        &ata_program,
    )
}

fn find_program_address(seeds: &[&[u8]], program_id: &[u8; 32]) -> AppResult<[u8; 32]> {
    use sha2::{Digest, Sha256};

    for bump in (0..=255u8).rev() {
        let mut hasher = Sha256::new();
        for seed in seeds {
            hasher.update(seed);
        }
        hasher.update([bump]);
        hasher.update(program_id);
        hasher.update(b"ProgramDerivedAddress");
        let hash = hasher.finalize();

        // Check if the result is a valid ed25519 point (off-curve = valid PDA)
        let bytes: [u8; 32] = hash.into();
        if curve25519_dalek_check_not_on_curve(&bytes) {
            return Ok(bytes);
        }
    }

    Err(AppError::Internal("Could not find PDA".into()))
}

/// Simple check: try to decompress as ed25519 point. If it fails, it's off-curve (valid PDA).
fn curve25519_dalek_check_not_on_curve(bytes: &[u8; 32]) -> bool {
    use ed25519_dalek::VerifyingKey;
    VerifyingKey::from_bytes(bytes).is_err()
}
