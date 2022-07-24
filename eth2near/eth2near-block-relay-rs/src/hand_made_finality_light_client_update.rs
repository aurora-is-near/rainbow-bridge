use std::error::Error;
use eth_types::eth2::LightClientUpdate;
use crate::beacon_rpc_client::BeaconRPCClient;

pub struct HandMadeFinalityLightClientUpdate {}

impl HandMadeFinalityLightClientUpdate {
    pub fn get_finality_light_client_update(beacon_rpc_client: &BeaconRPCClient,
                                            attested_slot: u64) -> Result<LightClientUpdate, Box<dyn Error>> {
        let attested_header = beacon_rpc_client.get_beacon_block_header_for_block_id(&format!("{}", attested_slot))?;
        Err("not implemented")?
    }
}

#[cfg(test)]
mod tests {
    const ATTESTED_HEADER_SLOT: u64 = 812637;

    #[test]
    fn test_hand_made_finality_light_client_update() {}
}
