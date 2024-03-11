use crate::contract_type::ContractType;
use contract_wrapper::eth_network::EthNetwork;
use contract_wrapper::near_network::NearNetwork;
use contract_wrapper::near_rpc_client::NearRPCClient;
use eth_rpc_client::beacon_rpc_client::BeaconRPCVersion;
use reqwest::Url;
use serde::Deserialize;
use std::io::Read;
use std::path::PathBuf;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    // endpoint to a full node of Eth2 Beacon chain with Light Client API
    pub beacon_endpoint: String,

    // endpoint for the Ethereum full node, which supports Eth1 RPC API
    pub eth1_endpoint: String,

    // the max number of headers submitted in one batch to eth client
    pub headers_batch_size: u32,

    // endpoint for a full node on the NEAR chain
    pub near_endpoint: String,

    // api key for the NEAR endpoint
    pub near_endpoint_api_key: Option<String>,

    // Account id from which relay make requests
    pub signer_account_id: String,

    // Path to the file with a secret key for signer account
    pub path_to_signer_secret_key: String,

    // Account id for eth client contract on NEAR
    pub contract_account_id: String,

    // The Ethereum network name (Mainnet, Kiln, Ropsten, Goerli)
    pub ethereum_network: EthNetwork,

    // Contract type (near, dao, file)
    pub contract_type: ContractType,

    // Period of submission light client updates. Once in N epochs.
    pub interval_between_light_client_updates_submission_in_epochs: u64,

    // maximum gap in slots between submitting light client update
    pub max_blocks_for_finalization: u64,

    // NEAR network name (mainnet, testnet)
    pub near_network_id: NearNetwork,

    // Port for Prometheus
    pub prometheus_metrics_port: Option<u16>,

    // Account id for DAO on NEAR
    pub dao_contract_account_id: Option<String>,

    // Path to dir for output submitted light client updates and execution blocks
    pub output_dir: Option<String>,

    // Path to the json file with beacon state in the next attested slot
    // for case of short relay run
    pub path_to_attested_state: Option<String>,

    // Include next sync committee to the Light Client Update in short relay run
    pub include_next_sync_committee_to_light_client: bool,

    // Timeout for ETH RPC requests in seconds
    pub eth_requests_timeout_seconds: u64,

    // Timeout for ETH RPC get status requests in seconds
    pub state_requests_timeout_seconds: u64,

    // Timeout for NEAR RPC requests in seconds
    pub near_requests_timeout_seconds: u64,

    // Sleep time in seconds when ETH client is synchronized with ETH network
    pub sleep_time_on_sync_secs: u64,

    // Sleep time in seconds after blocks/light_client_update submission to client
    pub sleep_time_after_submission_secs: u64,

    // Sleep time in seconds waiting for in-progress proposal to be processed
    pub sleep_time_on_in_progress_proposal_secs: u64,

    /// Max number of stored blocks in the storage of the eth2 client contract.
    /// Events that happen past this threshold cannot be verified by the client.
    /// It is used on initialization of the Eth2 client.
    pub hashes_gc_threshold: Option<u64>,

    /// Max number of unfinalized blocks allowed to be stored by one submitter account.
    /// It is used on initialization of the Eth2 client.
    pub max_submitted_blocks_by_account: Option<u32>,

    // Beacon rpc version (V1_1, V1_2)
    pub beacon_rpc_version: BeaconRPCVersion,

    pub get_light_client_update_by_epoch: Option<bool>,
}

impl Config {
    pub fn load_from_toml(path: PathBuf) -> Self {
        let mut config = std::fs::File::open(path).expect("Error on parsing path to config");
        let mut content = String::new();
        config
            .read_to_string(&mut content)
            .expect("Error on reading config");
        let config = toml::from_str(content.as_str()).expect("Error on config parsing");

        Self::check_urls(&config);
        Self::check_account_id(&config);

        config
    }

    fn check_urls(&self) {
        // check `beacon_endpoint`
        Url::parse(&self.beacon_endpoint).expect("Error on beacon endpoint URL parsing");

        // check `eth1_endpoint`
        Url::parse(&self.eth1_endpoint).expect("Error on ETH1 endpoint URL parsing");

        // check `near_endpoint`
        Url::parse(&self.near_endpoint).expect("Error on NEAR endpoint URL parsing");
    }

    fn check_account_id(&self) {
        let near_rpc_client = NearRPCClient::new(&self.near_endpoint, &self.near_endpoint_api_key);

        // check `signer_account_id`
        let _signer_account_id: near_sdk::AccountId = self
            .signer_account_id
            .parse()
            .expect("Error on signer account ID parsing");
        if !near_rpc_client
            .check_account_exists(&self.signer_account_id)
            .expect("Error on checking signer account ID existence")
        {
            panic!("Signer account id doesn't exist on NEAR network");
        }

        // check `contract_account_id`
        let _contract_account_id: near_sdk::AccountId = self
            .contract_account_id
            .parse()
            .expect("Error on contract account ID parsing");
        if !near_rpc_client
            .check_account_exists(&self.contract_account_id)
            .expect("Error on checking contract account ID existence")
        {
            panic!("Contract account id doesn't exist on NEAR network");
        }

        // check `dao_contract_account_id`
        if let Some(dao_contract_account_id) = self.dao_contract_account_id.clone() {
            let _dao_contract_account_id: near_sdk::AccountId = dao_contract_account_id
                .parse()
                .expect("Error on DAO contract account ID parsing");
            if !near_rpc_client
                .check_account_exists(&dao_contract_account_id)
                .expect("Error on checking DAO account ID existence")
            {
                panic!("DAO account id doesn't exist on NEAR network");
            }
        }
    }
}
