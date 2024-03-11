use clap::Parser;
use contract_wrapper::contract_wrapper_trait::ContractWrapper;
use contract_wrapper::eth_client_contract_trait::EthClientContractTrait;
use contract_wrapper::near_contract_wrapper::NearContractWrapper;
use contract_wrapper::{
    dao_contract, dao_eth_client_contract, eth_client_contract, file_eth_client_contract,
};
use eth2_to_near_relay::config::Config;
use eth2_to_near_relay::contract_type::ContractType;
use eth2_to_near_relay::eth2near_relay::Eth2NearRelay;
use eth2near_logger::SimpleLogger;
use log::LevelFilter;
use std::string::String;

#[derive(Parser, Default, Debug)]
#[clap(version, about = "Eth2 to Near Relay")]
struct Arguments {
    #[clap(short, long)]
    /// Path to config file
    config: String,

    #[clap(long, default_value_t = String::from("info"))]
    /// Log level (trace, debug, info, warn, error)
    log_level: String,
}

fn get_eth_contract_wrapper(config: &Config) -> Box<dyn ContractWrapper> {
    Box::new(NearContractWrapper::new(
        &config.near_endpoint,
        &config.signer_account_id,
        &config.path_to_signer_secret_key,
        &config.contract_account_id,
        Some(std::time::Duration::from_secs(
            config.near_requests_timeout_seconds,
        )),
        config.near_endpoint_api_key.clone(),
    ))
}

fn get_dao_contract_wrapper(config: &Config) -> Box<dyn ContractWrapper> {
    let dao_contract_account_id = config.dao_contract_account_id.clone();

    Box::new(NearContractWrapper::new(
        &config.near_endpoint,
        &config.signer_account_id,
        &config.path_to_signer_secret_key,
        &dao_contract_account_id
            .expect("No DAO contract account ID provided for relay running in DAO mode"),
        Some(std::time::Duration::from_secs(
            config.near_requests_timeout_seconds,
        )),
        config.near_endpoint_api_key.clone(),
    ))
}

fn get_eth_client_contract(config: &Config) -> Box<dyn EthClientContractTrait> {
    let eth_contract_wrapper = get_eth_contract_wrapper(config);
    let eth_client = eth_client_contract::EthClientContract::new(eth_contract_wrapper);

    match config.contract_type {
        ContractType::Dao => Box::new(dao_eth_client_contract::DaoEthClientContract::new(
            eth_client,
            dao_contract::DAOContract::new(get_dao_contract_wrapper(config)),
        )),
        ContractType::File => Box::new(file_eth_client_contract::FileEthClientContract::new(
            eth_client,
            config
                .output_dir
                .clone()
                .expect("No output dir provided for relay running in FILE mode"),
        )),
        ContractType::Near => Box::new(eth_client),
    }
}

fn init_log(args: &Arguments, config: &Config) {
    let log_level_filter = match args.log_level.to_lowercase().as_str() {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => LevelFilter::Info,
    };

    let mut path_to_log_file = "./eth2near-relay.log".to_string();
    if let Some(out_dir) = config.clone().output_dir {
        path_to_log_file = out_dir.clone() + "/" + "eth2near-relay.log";
        std::fs::create_dir_all(out_dir).expect("Error during output dir creation");
    }

    log::set_boxed_logger(Box::new(SimpleLogger::new(path_to_log_file)))
        .map(|()| log::set_max_level(log_level_filter))
        .expect("Error on logger creation");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Arguments::parse();
    let config = Config::load_from_toml(
        args.config
            .clone()
            .try_into()
            .expect("Error on config parsing"),
    );
    init_log(&args, &config);

    let mut eth2near_relay = Eth2NearRelay::init(&config, get_eth_client_contract(&config));

    eth2near_relay.run(None);
    Ok(())
}
