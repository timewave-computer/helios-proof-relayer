pub const LIGHT_CLIENT_PROVER_ENDPOINT: &str = "http://165.1.70.239:7778/";
#[allow(unused)]
pub const LIGHT_CLIENT_VK: &str =
    "0x006beadaace48146e0389403f70b490980e612c439a9294877446cd583e50fce";

#[allow(unused)]
pub const REGISTRY_ENDPOINT: &str =
    "http://prover.timewave.computer:37281/api/registry/domain/ethereum-alpha";

pub const LIGHT_CLIENT_MODE: MODE = MODE::HELIOS;

#[allow(unused)]
pub enum MODE {
    HELIOS,
    TENDERMINT,
}
