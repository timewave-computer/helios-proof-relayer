use anyhow::Result;
use hex;
use serde_json::json;
use sp1_sdk::SP1ProofWithPublicValues;
use std::time::Duration;
use tokio::time::sleep;

/// Endpoint for the Helios prover service
const HELIOS_PROVER_ENDPOINT: &str = "http://165.1.70.239:7778/";

/// Endpoint for the registry service
const REGISTRY_ENDPOINT: &str =
    "http://prover.timewave.computer:37281/api/registry/domain/ethereum-alpha";

pub async fn get_helios_block() -> Result<SP1ProofWithPublicValues, anyhow::Error> {
    let client = reqwest::Client::new();
    let response = client.get(HELIOS_PROVER_ENDPOINT).send().await.unwrap();
    let hex_str = response.text().await.unwrap();
    let bytes = hex::decode(hex_str)?;
    let state_proof: SP1ProofWithPublicValues = serde_json::from_slice(&bytes)?;
    Ok(state_proof)
}

pub async fn create_payload() -> Result<serde_json::Value, anyhow::Error> {
    let helios_block = get_helios_block().await?;
    let helios_block_proof = hex::encode(helios_block.bytes());
    let helios_block_public_values = hex::encode(helios_block.public_values.to_vec());
    let helios_block_vk = "0x006beadaace48146e0389403f70b490980e612c439a9294877446cd583e50fce";

    let payload = json!({
        "proof": helios_block_proof,
        "public_values": helios_block_public_values,
        "vk": helios_block_vk,
    });

    Ok(payload)
}

pub async fn send(payload: &serde_json::Value) -> Result<(), anyhow::Error> {
    println!("Payload: {:?}", payload);

    let client = reqwest::Client::new();
    let response = client.post(REGISTRY_ENDPOINT).json(payload).send().await?;

    println!("Response status: {}", response.status());
    let response_text = response.text().await?;
    println!("Response body: {}", response_text);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut previous_proof: Option<String> = None;

    loop {
        match create_payload().await {
            Ok(payload) => {
                // Extract the proof from the payload to compare
                let current_proof = payload["proof"].as_str().unwrap().to_string();

                // Check if this proof is different from the previous one
                let should_send = match &previous_proof {
                    None => {
                        println!("First run - sending initial proof");
                        true
                    }
                    Some(prev) => {
                        if prev != &current_proof {
                            println!("Proof has changed - sending updated proof");
                            true
                        } else {
                            println!("Proof unchanged - skipping POST request");
                            false
                        }
                    }
                };

                if should_send {
                    match send(&payload).await {
                        Ok(_) => {
                            println!("Successfully sent payload to registry");
                            previous_proof = Some(current_proof);
                        }
                        Err(e) => {
                            eprintln!("Failed to send payload to registry: {}", e);
                        }
                    }
                } else {
                    println!("Waiting for next check...");
                }
            }
            Err(e) => {
                eprintln!("Failed to create payload: {}", e);
            }
        }

        // Wait 30 seconds before checking again
        sleep(Duration::from_secs(30)).await;
    }
}

#[cfg(test)]
mod tests {
    use crate::create_payload;

    #[tokio::test]
    async fn test_get_latest_helios_block() {
        // get and validate a helios block
        let payload = create_payload().await.unwrap();
        println!("Payload: {:?}", payload);
    }
}
