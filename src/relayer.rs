#[allow(unused)]
use {
    crate::config::{LIGHT_CLIENT_PROVER_ENDPOINT, LIGHT_CLIENT_VK, REGISTRY_ENDPOINT},
    hex,
    serde_json::json,
    sp1_sdk::SP1ProofWithPublicValues,
    tracing::{debug, info},
};

pub async fn get_proof() -> Result<SP1ProofWithPublicValues, anyhow::Error> {
    info!("🔍 Fetching proof from {}", LIGHT_CLIENT_PROVER_ENDPOINT);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client.get(LIGHT_CLIENT_PROVER_ENDPOINT).send().await?;

    info!("📡 Received response with status: {}", response.status());

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "HTTP request failed with status: {}",
            response.status()
        ));
    }

    let hex_str = response.text().await?;
    info!("📦 Received hex string of length: {}", hex_str.len());

    let bytes = hex::decode(hex_str)?;
    let state_proof: SP1ProofWithPublicValues = serde_json::from_slice(&bytes)?;

    info!("✅ Successfully parsed proof");
    Ok(state_proof)
}

#[cfg(all(feature = "relayer", not(feature = "health-check")))]
pub async fn create_payload() -> Result<serde_json::Value, anyhow::Error> {
    let wrapper_proof = get_proof().await?;
    let wrapper_proof_encoded = hex::encode(wrapper_proof.bytes());
    let wrapper_proof_public_values_encoded = hex::encode(wrapper_proof.public_values.to_vec());

    let payload = json!({
        "proof": wrapper_proof_encoded,
        "public_values": wrapper_proof_public_values_encoded,
        "vk": LIGHT_CLIENT_VK,
    });

    Ok(payload)
}

#[cfg(all(feature = "relayer", not(feature = "health-check")))]
pub async fn send(payload: &serde_json::Value) -> Result<(), anyhow::Error> {
    debug!("Payload: {:?}", payload);

    let client = reqwest::Client::new();
    let response = client.post(REGISTRY_ENDPOINT).json(payload).send().await?;

    info!("Response status: {}", response.status());
    let response_text = response.text().await?;
    debug!("Response body: {}", response_text);

    Ok(())
}
