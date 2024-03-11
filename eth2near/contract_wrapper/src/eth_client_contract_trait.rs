use eth2_utility::types::ClientMode;
use eth_types::eth2::{LightClientState, LightClientUpdate};
use eth_types::{BlockHeader, H256};
use near_primitives::views::FinalExecutionOutcomeView;
use std::error::Error;

/// Interface for using Ethereum Light Client
pub trait EthClientContractTrait {
    /// Submits the Light Client Update to Ethereum Light Client on NEAR. Returns the final execution outcome or an error
    fn send_light_client_update(
        &mut self,
        light_client_update: LightClientUpdate,
    ) -> Result<FinalExecutionOutcomeView, Box<dyn Error>>;

    /// Check if the client is ready to accept new update
    fn is_ready_to_submit_light_client_update(&self) -> Result<bool, Box<dyn Error>>;

    /// Gets finalized beacon block hash from Ethereum Light Client on NEAR
    fn get_finalized_beacon_block_hash(&self) -> Result<H256, Box<dyn Error>>;

    /// Gets finalized beacon block slot from Ethereum Light Client on NEAR
    fn get_finalized_beacon_block_slot(&self) -> Result<u64, Box<dyn Error>>;

    /// Sends headers to Ethereum Light Client on NEAR. Returns final execution outcome or an error.
    ///
    /// # Arguments
    ///
    /// * `headers` - the list of headers for submission to Eth Client
    fn send_headers(
        &mut self,
        headers: &[BlockHeader],
    ) -> Result<FinalExecutionOutcomeView, Box<dyn std::error::Error>>;

    fn get_client_mode(&self) -> Result<ClientMode, Box<dyn Error>>;

    /// Gets the Light Client State of the Ethereum Light Client on NEAR
    fn get_light_client_state(&self) -> Result<LightClientState, Box<dyn Error>>;

    fn get_last_block_number(&self) -> Result<u64, Box<dyn Error>>;

    fn get_unfinalized_tail_block_number(&self) -> Result<Option<u64>, Box<dyn Error>>;
}
