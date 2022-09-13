use serde::Deserialize;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;
use crate::near_rpc_client::NearRPCClient;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    // endpoint to a full node of Eth2 Beacon chain with Light Client API
    pub beacon_endpoint: String,

    // endpoint for the Ethereum full node, which supports Eth1 RPC API
    pub eth1_endpoint: String,

    // the max number of headers submitted in one batch to eth client
    pub total_submit_headers: u32,

    // endpoint for a full node on the NEAR chain
    pub near_endpoint: String,

    // Account id from which relay make requests
    pub signer_account_id: String,

    // Path to the file with a secret key for signer account
    pub path_to_signer_secret_key: String,

    // Account id for eth client contract on NEAR
    pub contract_account_id: String,

    // The Ethereum network name (main, kiln)
    pub network: String,

    // Contract type (near, dao, file)
    pub contract_type: String,

    // Frequency of submission light client updates. Once in N epochs.
    pub light_client_updates_submission_frequency_in_epochs: u64,

    // maximum gap in slots between submitting light client update
    pub max_blocks_for_finalization: u64,

    // NEAR network name (mainnet, testnet)
    pub near_network_id: String,

    // Account id for DAO on NEAR
    pub dao_contract_account_id: Option<String>,

    // Path to dir for output submitted light client updates and execution blocks
    pub output_dir: Option<String>,

    // Path to the json file with beacon state in the next attested slot
    // for case of short relay run
    pub path_to_attested_state: Option<String>,

    // Path to the json file with beacon state in the next finality slot
    // for case of short relay run
    pub path_to_finality_state: Option<String>,

    //Timeout for ETH RPC requests in secs
    pub eth_requests_timeout: u64,

    //Timeout for ETH RPC get status requests in secs
    pub state_requests_timeout: u64,
}

impl Config {
    pub fn load_from_toml(path: PathBuf) -> Self {
        let mut config = std::fs::File::open(path).unwrap();
        let mut content = String::new();
        config.read_to_string(&mut content).unwrap();
        let config = toml::from_str(content.as_str()).unwrap();

        Self::check_urls(&config);
        Self::check_account_id(&config);
        Self::check_network_types(&config);
        Self::check_update_frequency(&config);

        config
    }

    fn check_urls(&self) {
        let timeout = Duration::new(5, 0);
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap();

        //check beacon_endpoint
        let response = client.head(&self.beacon_endpoint).send().unwrap();
        if !response.status().is_success() {
            panic!("Beacon_endpoint not available");
        }

        //check eth1_endpoint
        let response = client.head(&self.eth1_endpoint).send().unwrap();
        if !response.status().is_success() {
            panic!("Eth1_endpoint not available");
        }

        //check near_endpoint
        let response = client.head(&self.near_endpoint).send().unwrap();
        if !response.status().is_success() {
            panic!("Near_endpoint not available");
        }
    }

    fn check_account_id(&self) {
        let near_rpc_client = NearRPCClient::new(&self.near_endpoint);

        //check signer_account_id
        let _signer_account_id: near_sdk::AccountId = self.signer_account_id.parse().unwrap();
        if near_rpc_client.check_account_exists(&self.signer_account_id).unwrap() == false {
            panic!("Signer account id doesn't exist on NEAR network");
        }

        //check contract_account_id
        let _contract_account_id: near_sdk::AccountId = self.contract_account_id.parse().unwrap();
        if near_rpc_client.check_account_exists(&self.contract_account_id).unwrap() == false {
            panic!("Contract account id doesn't exist on NEAR network");
        }

        //check dao_contract_account_id
        if let Some(dao_contract_account_id) = self.dao_contract_account_id.clone() {
            let _dao_contract_account_id: near_sdk::AccountId = dao_contract_account_id.parse().unwrap();
            if near_rpc_client.check_account_exists(&dao_contract_account_id).unwrap() == false {
                panic!("DAO account id doesn't exist on NEAR network");
            }
        }
    }

    fn check_network_types(&self) {}
    fn check_update_frequency(&self) {}
}
