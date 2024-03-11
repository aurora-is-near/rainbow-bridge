use crate::contract_wrapper_trait::ContractWrapper;
use crate::eth_client_contract_trait::EthClientContractTrait;
use crate::eth_network::EthNetwork;
use borsh::BorshDeserialize;
use eth2_utility::types::ClientMode;
use eth_types::eth2::{
    ExtendedBeaconBlockHeader, LightClientState, LightClientUpdate, SyncCommittee,
};
use eth_types::{BlockHeader, H256};
use near_primitives::borsh::BorshSerialize;
use near_primitives::types::AccountId;
use near_primitives::views::FinalExecutionOutcomeView;
use serde::Serialize;
use serde_json::json;
use std::error::Error;
use std::option::Option;
use std::string::String;

/// Implementation for interaction with Ethereum Light Client Contract on NEAR.
pub struct EthClientContract {
    /// Wrapper for interacting with NEAR Contract
    pub contract_wrapper: Box<dyn ContractWrapper>,
}

impl EthClientContract {
    /// Constructor for `EthClientContract`
    pub fn new(contract_wrapper: Box<dyn ContractWrapper>) -> Self {
        EthClientContract { contract_wrapper }
    }

    /// Initializes the Ethereum Light Client Contract on NEAR.
    ///
    /// # Arguments
    /// * `network` - the name of Ethereum network such as `mainnet`, `goerli`, `kiln`, etc.
    /// * `finalized_execution_header` - the finalized execution header to start initialization with.
    /// * `finalized_beacon_header` - correspondent finalized beacon header.
    /// * `current_sync_committee` - sync committee correspondent for finalized block.
    /// * `next_sync_committee` - sync committee for the next period after period for finalized block.
    /// * `hashes_gs_threshold` - the maximum number of stored finalized blocks.
    /// * `max_submitted_block_by_account` - the maximum number of unfinalized blocks which one relay can store in the client's storage.
    /// * `trusted_signer` - the account address of the trusted signer which is allowed to submit light client updates.
    pub fn init_contract(
        &self,
        ethereum_network: EthNetwork,
        finalized_execution_header: BlockHeader,
        finalized_beacon_header: ExtendedBeaconBlockHeader,
        current_sync_committee: SyncCommittee,
        next_sync_committee: SyncCommittee,
        validate_updates: Option<bool>,
        verify_bls_signatures: Option<bool>,
        hashes_gc_threshold: Option<u64>,
        trusted_signer: Option<AccountId>,
    ) {
        #[derive(BorshSerialize, Serialize)]
        pub struct InitInput {
            pub network: String,
            pub finalized_execution_header: eth_types::BlockHeader,
            pub finalized_beacon_header: ExtendedBeaconBlockHeader,
            pub current_sync_committee: SyncCommittee,
            pub next_sync_committee: SyncCommittee,
            pub validate_updates: bool,
            pub verify_bls_signatures: bool,
            pub hashes_gc_threshold: u64,
            pub trusted_signer: Option<AccountId>,
        }

        let init_input = InitInput {
            network: ethereum_network.to_string(),
            finalized_execution_header,
            finalized_beacon_header,
            current_sync_committee,
            next_sync_committee,
            validate_updates: validate_updates.unwrap_or(true),
            verify_bls_signatures: verify_bls_signatures.unwrap_or(false),
            hashes_gc_threshold: hashes_gc_threshold.unwrap_or(51_000),
            trusted_signer,
        };

        println!(
            "Init eth2 client input: \n {}",
            serde_json::to_string_pretty(&init_input).unwrap()
        );

        self.contract_wrapper
            .call_change_method(
                "init".to_string(),
                init_input.try_to_vec().expect("Error on parse init_input"),
                None,
                None,
            )
            .expect("Error during contract initialization");
    }

    /// Returns the Eth Light Client account address
    pub fn get_account_id(&self) -> AccountId {
        self.contract_wrapper.get_account_id()
    }

    pub fn get_signer_account_id(&self) -> AccountId {
        self.contract_wrapper.get_signer_account_id()
    }
}

impl EthClientContractTrait for EthClientContract {
    fn send_light_client_update(
        &mut self,
        light_client_update: LightClientUpdate,
    ) -> Result<FinalExecutionOutcomeView, Box<dyn Error>> {
        self.contract_wrapper.call_change_method(
            "submit_beacon_chain_light_client_update".to_string(),
            light_client_update.try_to_vec()?,
            None,
            None,
        )
    }

    fn is_ready_to_submit_light_client_update(&self) -> Result<bool, Box<dyn Error>> {
        Ok(true)
    }

    fn get_finalized_beacon_block_hash(&self) -> Result<H256, Box<dyn Error>> {
        let result = self.contract_wrapper.call_view_function(
            "finalized_beacon_block_root".to_string(),
            json!({}).to_string().into_bytes(),
        )?;
        let beacon_block_hash: H256 = H256::try_from_slice(&result)?;
        Ok(beacon_block_hash)
    }

    fn get_finalized_beacon_block_slot(&self) -> Result<u64, Box<dyn Error>> {
        let result = self.contract_wrapper.call_view_function(
            "finalized_beacon_block_slot".to_string(),
            json!({}).to_string().into_bytes(),
        )?;
        let beacon_block_slot: u64 = u64::try_from_slice(&result)?;
        Ok(beacon_block_slot)
    }

    fn get_client_mode(&self) -> Result<ClientMode, Box<dyn Error>> {
        let res = self.contract_wrapper.call_view_function(
            "get_client_mode".to_string(),
            json!({}).to_string().into_bytes(),
        )?;

        let mode: ClientMode = ClientMode::try_from_slice(&res)?;
        Ok(mode)
    }

    fn send_headers(
        &mut self,
        headers: &[BlockHeader],
    ) -> Result<FinalExecutionOutcomeView, Box<dyn std::error::Error>> {
        if headers.len() == 0 {
            return Err(Box::new(crate::errors::TryToSubmitZeroHeaderError));
        }

        let method_names = vec!["submit_execution_header".to_string(); headers.len()];
        let args = headers
            .iter()
            .filter_map(|header| header.try_to_vec().ok())
            .collect();

        self.contract_wrapper
            .call_change_method_batch(method_names, args, None, None)
    }

    fn get_light_client_state(&self) -> Result<LightClientState, Box<dyn Error>> {
        let result = self
            .contract_wrapper
            .call_view_function("get_light_client_state".to_string(), vec![])?;

        Ok(LightClientState::try_from_slice(result.as_slice())?)
    }

    fn get_last_block_number(&self) -> Result<u64, Box<dyn Error>> {
        let response = self.contract_wrapper.call_view_function(
            "last_block_number".to_string(),
            json!({}).to_string().into_bytes(),
        )?;

        let beacon_block_number: u64 = u64::try_from_slice(&response)?;
        Ok(beacon_block_number)
    }

    fn get_unfinalized_tail_block_number(&self) -> Result<Option<u64>, Box<dyn Error>> {
        let response = self.contract_wrapper.call_view_function(
            "get_unfinalized_tail_block_number".to_string(),
            json!({}).to_string().into_bytes(),
        )?;

        let beacon_block_number: Option<u64> = Option::<u64>::try_from_slice(&response)?;
        Ok(beacon_block_number)
    }
}

#[cfg(test)]
mod tests {
    use crate::eth_client_contract;
    use crate::eth_client_contract::EthClientContract;
    use crate::eth_client_contract_trait::EthClientContractTrait;
    use crate::sandbox_contract_wrapper::SandboxContractWrapper;
    use eth_types::eth2::{ExtendedBeaconBlockHeader, LightClientUpdate, SyncCommittee};
    use eth_types::BlockHeader;
    use near_primitives::types::AccountId;
    use tokio::runtime::Runtime;

    // TODO: use a more clean approach to include binary
    const WASM_FILEPATH: &str = "../../contracts/near/res/eth2_client.wasm";

    struct EthState {
        pub execution_blocks: Vec<BlockHeader>,
        pub light_client_updates: Vec<LightClientUpdate>,
        pub current_execution_block: usize,
        pub current_light_client_update: usize,
    }

    impl EthState {
        pub fn new() -> Self {
            const PATH_TO_EXECUTION_BLOCKS: &str =
                "./data/execution_block_headers_goerli_5262172-5262492.json";
            const PATH_TO_LIGHT_CLIENT_UPDATES: &str =
                "./data/light_client_updates_goerli_5262172-5262492.json";

            let execution_blocks: Vec<BlockHeader> = serde_json::from_str(
                &std::fs::read_to_string(PATH_TO_EXECUTION_BLOCKS).expect("Unable to read file"),
            )
            .unwrap();

            let light_client_updates: Vec<LightClientUpdate> = serde_json::from_str(
                &std::fs::read_to_string(PATH_TO_LIGHT_CLIENT_UPDATES)
                    .expect("Unable to read file"),
            )
            .unwrap();

            Self {
                execution_blocks,
                light_client_updates,
                current_execution_block: 0,
                current_light_client_update: 0,
            }
        }

        pub fn submit_block(&mut self, eth_client: &mut EthClientContract) {
            eth_client
                .send_headers(&vec![
                    self.execution_blocks[self.current_execution_block].clone()
                ])
                .unwrap();

            self.current_execution_block += 1;
            while self.execution_blocks[self.current_execution_block].hash
                == self.execution_blocks[self.current_execution_block - 1].hash
            {
                self.current_execution_block += 1;
            }
        }

        pub fn submit_update(&mut self, eth_client: &mut EthClientContract) {
            eth_client
                .send_light_client_update(
                    self.light_client_updates[self.current_light_client_update].clone(),
                )
                .unwrap();
            self.current_light_client_update += 1;
        }
    }

    fn create_contract() -> (workspaces::Account, workspaces::Contract) {
        let rt = Runtime::new().unwrap();

        let worker = rt.block_on(workspaces::sandbox()).unwrap();
        let wasm = std::fs::read(WASM_FILEPATH).unwrap();

        // create accounts
        let owner = worker.root_account().unwrap();
        let relay_account = rt
            .block_on(
                owner
                    .create_subaccount("relay_account")
                    .initial_balance(30 * near_sdk::ONE_NEAR)
                    .transact(),
            )
            .unwrap()
            .into_result()
            .unwrap();

        let contract = rt.block_on(owner.deploy(&wasm)).unwrap().unwrap();

        (relay_account, contract)
    }

    fn init_contract(
        eth_client_contract: &EthClientContract,
        eth_state: &mut EthState,
        trusted_signer: String,
    ) {
        const PATH_TO_CURRENT_SYNC_COMMITTEE: &str =
            "./data/next_sync_committee_goerli_period_641.json";
        const PATH_TO_NEXT_SYNC_COMMITTEE: &str =
            "./data/next_sync_committee_goerli_period_642.json";

        let current_sync_committee: SyncCommittee = serde_json::from_str(
            &std::fs::read_to_string(PATH_TO_CURRENT_SYNC_COMMITTEE).expect("Unable to read file"),
        )
        .unwrap();
        let next_sync_committee: SyncCommittee = serde_json::from_str(
            &std::fs::read_to_string(PATH_TO_NEXT_SYNC_COMMITTEE).expect("Unable to read file"),
        )
        .unwrap();

        let finalized_beacon_header = ExtendedBeaconBlockHeader::from(
            eth_state.light_client_updates[0]
                .clone()
                .finality_update
                .header_update,
        );

        let finalized_hash = eth_state.light_client_updates[0]
            .clone()
            .finality_update
            .header_update
            .execution_block_hash;
        let mut finalized_execution_header = None::<BlockHeader>;
        for header in &eth_state.execution_blocks {
            eth_state.current_execution_block += 1;
            if header.hash.unwrap() == finalized_hash {
                finalized_execution_header = Some(header.clone());
                break;
            }
        }

        eth_client_contract.init_contract(
            eth_client_contract::EthNetwork::Goerli,
            finalized_execution_header.unwrap(),
            finalized_beacon_header,
            current_sync_committee,
            next_sync_committee,
            Some(true),
            Some(false),
            None,
            Option::<AccountId>::Some(trusted_signer.parse().unwrap()),
        );
        eth_state.current_light_client_update = 1;
    }

    #[test]
    fn test_smoke_eth_client_contract_wrapper() {
        let (relay_account, contract) = create_contract();

        // Use contract with `contract` as a signer for the `init()` call
        let contract_wrapper = Box::new(SandboxContractWrapper::new(
            contract.as_account(),
            contract.clone(),
        ));
        let eth_client_contract = eth_client_contract::EthClientContract::new(contract_wrapper);

        let mut eth_state = EthState::new();

        init_contract(
            &eth_client_contract,
            &mut eth_state,
            relay_account.id().to_string(),
        );
        let first_finalized_slot = eth_client_contract
            .get_finalized_beacon_block_slot()
            .unwrap();
        assert_eq!(first_finalized_slot, 5262208);

        // Use `relay_account` as a signer for normal operations
        let contract_wrapper = Box::new(SandboxContractWrapper::new(&relay_account, contract));
        let mut eth_client_contract = eth_client_contract::EthClientContract::new(contract_wrapper);

        let next_hash = eth_state.light_client_updates[eth_state.current_light_client_update]
            .clone()
            .finality_update
            .header_update
            .execution_block_hash;
        loop {
            let current_execution_block_hash = eth_state.execution_blocks
                [eth_state.current_execution_block]
                .hash
                .unwrap();

            eth_state.submit_block(&mut eth_client_contract);

            if current_execution_block_hash == next_hash {
                eth_state.submit_update(&mut eth_client_contract);
                let current_finality_slot = eth_client_contract
                    .get_finalized_beacon_block_slot()
                    .unwrap();
                assert_ne!(current_finality_slot, first_finalized_slot);
                break;
            }
        }
    }
}
