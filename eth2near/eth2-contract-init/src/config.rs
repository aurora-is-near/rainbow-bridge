use contract_wrapper::eth_network_enum::EthNetwork;
use contract_wrapper::near_network_enum::NearNetwork;
use contract_wrapper::near_rpc_client::NearRPCClient;
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

    // endpoint for a full node on the NEAR chain
    pub near_endpoint: String,

    // Account id from which relay make requests
    pub signer_account_id: String,

    // Path to the file with a secret key for signer account
    pub path_to_signer_secret_key: String,

    // Account id for eth client contract on NEAR
    pub contract_account_id: String,

    // The Ethereum network name (mainnet, kiln, ropsten, goerli)
    pub ethereum_network: EthNetwork,

    // NEAR network name (mainnet, testnet)
    pub near_network_id: NearNetwork,

    // Path to dir for output submitted light client updates and execution blocks
    pub output_dir: Option<String>,

    // Timeout for ETH RPC requests in seconds
    pub eth_requests_timeout_seconds: u64,

    pub validate_updates: bool,

    pub verify_bls_signature: bool,

    pub hashes_gc_threshold: u64,

    pub max_submitted_blocks_by_account: u32,

    pub trusted_signature: Option<String>,

    /// The trusted block root for checkpoint for contract initialization
    /// e.g.: 0x9cd0c5a8392d0659426b12384e8440c147510ab93eeaeccb08435a462d7bb1c7
    pub init_block_root: Option<String>,
}

impl Config {
    pub fn load_from_toml(path: PathBuf) -> Self {
        let mut config = std::fs::File::open(path).expect("Error on opening file with config");
        let mut content = String::new();
        config.read_to_string(&mut content).expect("Error on reading config");
        let config = toml::from_str(content.as_str()).expect("Error on parse config");

        Self::check_urls(&config);
        Self::check_account_id(&config);

        config
    }

    fn check_urls(&self) {
        // check `beacon_endpoint`
        Url::parse(&self.beacon_endpoint).expect("Incorrect beacon endpoint");

        // check `eth1_endpoint`
        Url::parse(&self.eth1_endpoint).expect("Incorrect eth1 endpoint");

        // check `near_endpoint`
        Url::parse(&self.near_endpoint).expect("Incorrect near endpoint");
    }

    fn check_account_id(&self) {
        let near_rpc_client = NearRPCClient::new(&self.near_endpoint);

        // check `signer_account_id`
        self.signer_account_id.parse::<near_sdk::AccountId>().expect("Incorrect signature account");
        if !near_rpc_client
            .check_account_exists(&self.signer_account_id)
            .expect("Error on checking signature account existence")
        {
            panic!("Signer account id doesn't exist on NEAR network");
        }

        // check `trusted_signature`
        if let Some(trusted_signature) = self.trusted_signature.clone() {
            trusted_signature.parse::<near_sdk::AccountId>().expect("Incorrect contract account id");
            if !near_rpc_client
                .check_account_exists(&trusted_signature)
                .expect("Error on checking trusted signature account existence")
            {
                panic!("Trusted signature doesn't exist on NEAR network");
            }
        }

        // check `contract_account_id`
        self.contract_account_id.parse::<near_sdk::AccountId>().expect("Incorrect contract account id");
        if !near_rpc_client
            .check_account_exists(&self.contract_account_id)
            .expect("Error on checking contract account existence")
        {
            panic!("Contract account id doesn't exist on NEAR network");
        }
    }
}
