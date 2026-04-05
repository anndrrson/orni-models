use std::sync::Arc;
use std::time::Duration;

use crate::state::AppState;

/// Run the settlement loop — every 5 minutes, batch pending payouts and send USDC on-chain.
pub async fn settlement_loop(state: Arc<AppState>) {
    let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes

    loop {
        interval.tick().await;

        if state.escrow_keypair.is_none() {
            continue; // No escrow keypair configured — skip settlement
        }

        if let Err(e) = process_settlements(&state).await {
            tracing::error!("Settlement error: {e}");
        }
    }
}

async fn process_settlements(state: &AppState) -> anyhow::Result<()> {
    // 1. Get all pending settlements grouped by creator wallet
    let pending: Vec<(String, i64)> = sqlx::query_as(
        r#"SELECT creator_wallet, SUM(amount_micro_usdc)::BIGINT as total
           FROM settlement_queue
           WHERE status = 'pending' AND creator_wallet IS NOT NULL AND creator_wallet != ''
           GROUP BY creator_wallet"#,
    )
    .fetch_all(&state.db)
    .await?;

    if pending.is_empty() {
        return Ok(());
    }

    tracing::info!("Processing {} settlement batches", pending.len());

    let keypair_bytes = state.escrow_keypair.as_ref().unwrap();
    let devnet = state.config.solana_rpc_url.contains("devnet");

    let client = said_solana::SolanaClient::new(&state.config.solana_rpc_url, keypair_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to create Solana client: {e}"))?;

    for (wallet, total_micro_usdc) in &pending {
        if *total_micro_usdc <= 0 {
            continue;
        }

        // Decode wallet address to bytes
        let wallet_bytes: [u8; 32] = match bs58::decode(wallet).into_vec() {
            Ok(v) if v.len() == 32 => v.try_into().unwrap(),
            _ => {
                tracing::warn!("Invalid wallet address: {wallet}, skipping");
                sqlx::query(
                    "UPDATE settlement_queue SET status = 'failed' WHERE creator_wallet = $1 AND status = 'pending'",
                )
                .bind(wallet)
                .execute(&state.db)
                .await?;
                continue;
            }
        };

        // Mark as processing
        sqlx::query(
            "UPDATE settlement_queue SET status = 'processing' WHERE creator_wallet = $1 AND status = 'pending'",
        )
        .bind(wallet)
        .execute(&state.db)
        .await?;

        // Send USDC on-chain
        match client.transfer_usdc(&wallet_bytes, *total_micro_usdc as u64, devnet).await {
            Ok(tx_sig) => {
                tracing::info!(
                    wallet = %wallet,
                    amount = %total_micro_usdc,
                    tx = %tx_sig,
                    "Settlement sent"
                );

                // Mark as settled with tx signature
                sqlx::query(
                    "UPDATE settlement_queue SET status = 'settled', tx_signature = $1, settled_at = NOW() WHERE creator_wallet = $2 AND status = 'processing'",
                )
                .bind(&tx_sig)
                .bind(wallet)
                .execute(&state.db)
                .await?;
            }
            Err(e) => {
                tracing::error!(wallet = %wallet, error = %e, "Settlement transfer failed");

                // Revert to pending for retry
                sqlx::query(
                    "UPDATE settlement_queue SET status = 'pending' WHERE creator_wallet = $1 AND status = 'processing'",
                )
                .bind(wallet)
                .execute(&state.db)
                .await?;
            }
        }
    }

    Ok(())
}
