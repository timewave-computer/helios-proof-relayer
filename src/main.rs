use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

use crate::api::{create_api_server, start_api_server};
use crate::config::{LIGHT_CLIENT_MODE, MODE};
use crate::db::{Database, HealthCheckData, PreviousProof};
use crate::relayer::get_proof;
#[cfg(all(feature = "relayer", not(feature = "health-check")))]
use crate::relayer::{create_payload, send};
mod api;
mod config;
mod db;
mod relayer;

use helios_recursion_types::WrapperCircuitOutputs as HeliosWrapperCircuitOutputs;
use tendermint_recursion_types::WrapperCircuitOutputs as TendermintWrapperCircuitOutputs;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize tracing subscriber with proper configuration
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .init();

    info!("🚀 Starting Helios Proof Relayer...");

    #[cfg(all(feature = "relayer", not(feature = "health-check")))]
    {
        info!("📡 Running in relayer mode");
        // Initialize database
        let db = std::sync::Arc::new(Database::new("relayer.db")?);

        // Load previous proof from database if it exists
        let mut previous_proof: Option<String> = match db.get_previous_proof()? {
            Some(proof) => Some(proof.proof_data),
            None => None,
        };

        // Start the relayer loop
        loop {
            match create_payload().await {
                Ok(payload) => {
                    // Extract the proof from the payload to compare
                    let current_proof = payload["proof"].as_str().unwrap().to_string();

                    // Check if this proof is different from the previous one
                    let should_send = match &previous_proof {
                        None => true,
                        Some(prev) => {
                            if prev != &current_proof {
                                true
                            } else {
                                false
                            }
                        }
                    };

                    if should_send {
                        match send(&payload).await {
                            Ok(_) => {
                                info!("✅ Successfully sent payload to registry");
                                previous_proof = Some(current_proof.clone());

                                // Store the new proof in database
                                let proof_data = PreviousProof {
                                    proof_data: current_proof,
                                    timestamp: chrono::Utc::now(),
                                };
                                if let Err(e) = db.update_previous_proof(&proof_data) {
                                    error!("❌ Failed to update previous proof in database: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("❌ Failed to send payload to registry: {}", e);
                            }
                        }
                    } else {
                        info!("⏳ Waiting for next check...");
                    }
                }
                Err(e) => {
                    error!("❌ Failed to create payload: {}", e);
                }
            }
            sleep(Duration::from_secs(30)).await;
        }
    }

    #[cfg(any(feature = "health-check", not(feature = "relayer")))]
    {
        info!("🏥 Running in health-check mode");

        // Initialize database
        info!("💾 Initializing database...");
        let db = std::sync::Arc::new(Database::new("health_check.db")?);
        info!("✅ Database initialized successfully");

        // Clear database for testing
        info!("🧹 Clearing database tables for fresh start...");
        if let Err(e) = db.clear_all_tables() {
            warn!("⚠️  Failed to clear database tables: {}", e);
        } else {
            info!("✅ Database tables cleared successfully");
        }

        // Create API server
        info!("🌐 Creating API server...");
        let api_router = create_api_server(db.clone());
        info!("✅ API server created");

        // Start the health check loop in a separate task
        info!("🔍 Starting health check service...");
        let health_check_handle = tokio::spawn(async move {
            info!("✅ Health check service started");

            loop {
                info!("🔍 Fetching latest proof...");
                match get_proof().await {
                    Ok(proof) => {
                        info!("✅ Proof fetched successfully");

                        // Get previous proof from database
                        let previous_proof = match db.get_previous_proof() {
                            Ok(Some(prev)) => Some(prev.proof_data),
                            Ok(None) => None,
                            Err(e) => {
                                warn!("⚠️  Error getting previous proof from database: {}", e);
                                None
                            }
                        };

                        // Check if proof has changed
                        let current_proof_hex = hex::encode(proof.bytes());
                        let should_update = match &previous_proof {
                            None => {
                                info!("🆕 No previous proof found, processing new proof");
                                true
                            }
                            Some(prev) => {
                                if prev != &current_proof_hex {
                                    info!("🔄 Proof has changed, processing new proof");
                                    true
                                } else {
                                    info!("⏳ Proof unchanged, skipping update");
                                    sleep(Duration::from_secs(120)).await;
                                    continue;
                                }
                            }
                        };

                        if should_update {
                            let mut current_height: u64 = 0;
                            let mut current_root: [u8; 32] = [0; 32];

                            match LIGHT_CLIENT_MODE {
                                MODE::HELIOS => {
                                    let public_outputs: HeliosWrapperCircuitOutputs =
                                        borsh::from_slice(&proof.public_values.as_slice()).unwrap();
                                    current_height = public_outputs.height;
                                    current_root = public_outputs.root;
                                }
                                MODE::TENDERMINT => {
                                    let public_outputs: TendermintWrapperCircuitOutputs =
                                        borsh::from_slice(proof.public_values.as_slice()).unwrap();
                                    current_height = public_outputs.height;
                                    current_root = public_outputs.root;
                                }
                            }

                            info!(
                                "📊 Processing proof - Height: {}, Root: {}",
                                current_height,
                                hex::encode(current_root)
                            );

                            // Store health check data in database when proof changes
                            let health_data = HealthCheckData {
                                current_height,
                                current_root: current_root.to_vec(),
                                timestamp: chrono::Utc::now(),
                            };

                            if let Err(e) = db.update_health_check(&health_data) {
                                error!("❌ Failed to update health check data in database: {}", e);
                            } else {
                                info!(
                                    "💾 Health check data updated - Height: {}, Root: {}",
                                    current_height,
                                    hex::encode(current_root)
                                );
                            }

                            // Store the new proof in database
                            let proof_data = PreviousProof {
                                proof_data: current_proof_hex,
                                timestamp: chrono::Utc::now(),
                            };
                            if let Err(e) = db.update_previous_proof(&proof_data) {
                                error!("❌ Failed to update previous proof in database: {}", e);
                            } else {
                                info!("💾 Proof stored in database");
                            }

                            info!("⏰ Waiting 120 seconds before next check...");
                        }
                    }
                    Err(e) => {
                        error!("❌ Health check failed: {}", e);
                    }
                }
                // Wait 2 minutes before next health check
                sleep(Duration::from_secs(120)).await;
            }
        });

        // Start the API server in a separate task
        info!("🌐 Starting API server...");
        let api_handle = tokio::spawn(async move {
            info!("✅ API server started");
            if let Err(e) = start_api_server(api_router).await {
                error!("❌ API server error: {}", e);
            }
        });

        info!("🔄 Waiting for services to complete...");
        // Wait for both tasks to conclude
        let (health_check_result, api_result) = tokio::join!(health_check_handle, api_handle);

        // Handle any errors from the tasks
        if let Err(e) = health_check_result {
            error!("❌ Health check service crashed: {}", e);
            return Err(anyhow::anyhow!("{}", e));
        }

        if let Err(e) = api_result {
            error!("❌ API server crashed: {}", e);
            return Err(anyhow::anyhow!("{}", e));
        }
    }

    Ok(())
}

#[cfg(test)]
#[cfg(all(feature = "relayer", not(feature = "health-check")))]
mod tests {
    use crate::create_payload;

    #[tokio::test]
    async fn test_get_latest_helios_block() {
        // get and validate a helios block
        let payload = create_payload().await.unwrap();
        info!("Payload: {:?}", payload);
    }
}
