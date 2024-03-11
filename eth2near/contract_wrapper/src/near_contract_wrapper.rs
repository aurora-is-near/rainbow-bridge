use crate::contract_wrapper_trait::ContractWrapper;
use crate::utils;
use near_crypto::InMemorySigner;
use near_jsonrpc_client::{methods, JsonRpcClient};
use near_jsonrpc_primitives::types::query::QueryResponseKind;
use near_primitives::transaction::{Action, FunctionCallAction, Transaction};
use near_primitives::types::{AccountId, BlockReference, Finality, FunctionArgs};
use near_primitives::views::{FinalExecutionOutcomeView, QueryRequest};
use near_sdk::{Balance, Gas};
use serde_json::Value;
use std::error::Error;
use std::string::String;
use std::time;
use std::vec::Vec;
use tokio::runtime::Runtime;

pub const MAX_GAS: Gas = Gas(Gas::ONE_TERA.0 * 300);
pub const DEFAULT_WAIT_FINAL_OUTCOME_TIMEOUT_SEC: u64 = 500;

/// Implementation of interaction with a contract on NEAR.
pub struct NearContractWrapper {
    /// RPC client for interaction with NEAR RPC endpoint
    client: JsonRpcClient,

    /// Account ID of the contract
    contract_account: AccountId,

    /// Account that signs the transactions
    signer: InMemorySigner,
}

impl NearContractWrapper {
    /// Constructor of `NearContractorWrapper`
    ///
    /// # Arguments
    ///
    /// * `near_endpoint` - The URL to NEAR RPC endpoint.
    /// * `account_id` - Signer account ID.
    /// * `signer_secret_key` - Signer secret key as the raw string.
    /// * `contract_account_id` - Contract account ID.
    pub fn new_with_raw_secret_key(
        near_endpoint: &str,
        account_id: &str,
        signer_secret_key: &str,
        contract_account_id: &str,
        timeout: Option<std::time::Duration>,
        api_key: Option<String>,
    ) -> NearContractWrapper {
        let signer_account_id = account_id
            .parse()
            .expect("Error on parsing account id during creation near contract wrapper");
        let mut client =
            JsonRpcClient::with(utils::new_near_rpc_client(timeout)).connect(near_endpoint);

        if let Some(api_key) = api_key {
            client = client.header(near_jsonrpc_client::auth::ApiKey::new(api_key).unwrap());
        }

        let contract_account = contract_account_id
            .parse()
            .expect("Error on parsing contract account id during creation near contract wrapper");

        let signer = InMemorySigner::from_secret_key(
            signer_account_id,
            signer_secret_key
                .parse()
                .expect("Error on parsing signature secret key"),
        );

        NearContractWrapper {
            client,
            contract_account,
            signer,
        }
    }

    /// Constructor of `NearContractorWrapper`
    ///
    /// # Arguments
    ///
    /// * `near_endpoint` - The URL to NEAR RPC endpoint.
    /// * `account_id` - Signer account ID.
    /// * `path_to_signer_secret_key` - Path to the file containing signer's secret key file.
    /// * `contract_account_id` - Contract account ID.
    pub fn new(
        near_endpoint: &str,
        account_id: &str,
        path_to_signer_secret_key: &str,
        contract_account_id: &str,
        timeout: Option<std::time::Duration>,
        api_key: Option<String>,
    ) -> NearContractWrapper {
        let v: Value = serde_json::from_str(
            &std::fs::read_to_string(path_to_signer_secret_key).expect("Unable to read file"),
        )
        .expect("Error on parsing file with secret key during contract initialization");
        let signer_secret_key = utils::trim_quotes(
            serde_json::to_string(&v["private_key"])
                .expect("Error during trim quotes of signature secret key"),
        );

        Self::new_with_raw_secret_key(
            near_endpoint,
            account_id,
            &signer_secret_key,
            contract_account_id,
            timeout,
            api_key,
        )
    }
}

impl ContractWrapper for NearContractWrapper {
    fn get_account_id(&self) -> AccountId {
        self.contract_account.clone()
    }

    fn get_signer_account_id(&self) -> AccountId {
        self.signer.account_id.clone()
    }

    fn call_view_function(
        &self,
        method_name: String,
        args: Vec<u8>,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let rt = Runtime::new()?;
        let handle = rt.handle();

        let request = methods::query::RpcQueryRequest {
            block_reference: BlockReference::Finality(Finality::Final),
            request: QueryRequest::CallFunction {
                account_id: self.contract_account.clone(),
                method_name,
                args: FunctionArgs::from(args),
            },
        };

        let response = handle.block_on(self.client.call(request))?;

        if let QueryResponseKind::CallResult(result) = response.kind {
            Ok(result.result)
        } else {
            Err("view method doesn't return any result")?
        }
    }

    fn call_change_method_batch(
        &self,
        method_name: Vec<String>,
        args: Vec<Vec<u8>>,
        deposit: Option<Vec<Balance>>,
        gas: Option<Gas>,
    ) -> Result<FinalExecutionOutcomeView, Box<dyn Error>> {
        let rt = Runtime::new()?;

        let access_key_query_response =
            rt.block_on(self.client.call(methods::query::RpcQueryRequest {
                block_reference: BlockReference::latest(),
                request: near_primitives::views::QueryRequest::ViewAccessKey {
                    account_id: self.signer.account_id.clone(),
                    public_key: self.signer.public_key.clone(),
                },
            }))?;

        let current_nonce = match access_key_query_response.kind {
            QueryResponseKind::AccessKey(access_key) => access_key.nonce,
            _ => Err("failed to extract current nonce")?,
        };

        let num_blocks_in_batch = method_name.len() as u64;

        if num_blocks_in_batch == 0 {
            return Err(Box::new(crate::errors::TryToSubmitZeroHeaderError));
        }

        let attached_gas_per_promise_in_batch = gas.unwrap_or(MAX_GAS) / num_blocks_in_batch;
        let mut actions = Vec::new();

        for i in 0..method_name.len() {
            actions.push(Action::FunctionCall(FunctionCallAction {
                method_name: method_name[i].clone(),
                args: args[i].clone(),
                gas: attached_gas_per_promise_in_batch.0,
                deposit: deposit.as_ref().map(|d| d[i]).unwrap_or(0),
            }));
        }

        let transaction = Transaction {
            signer_id: self.signer.account_id.clone(),
            public_key: self.signer.public_key.clone(),
            nonce: current_nonce + 1,
            receiver_id: self.contract_account.clone(),
            block_hash: access_key_query_response.block_hash,
            actions,
        };

        let request = methods::broadcast_tx_async::RpcBroadcastTxAsyncRequest {
            signed_transaction: transaction.sign(&self.signer),
        };

        let hash = rt.block_on(self.client.call(&request))?;
        let sent_at = time::Instant::now();
        let tx_info = methods::tx::TransactionInfo::TransactionId {
            hash,
            account_id: self.signer.account_id.clone(),
        };

        loop {
            let response =
                rt.block_on(self.client.call(methods::tx::RpcTransactionStatusRequest {
                    transaction_info: tx_info.clone(),
                }));

            let delta = (time::Instant::now() - sent_at).as_secs();
            if delta > DEFAULT_WAIT_FINAL_OUTCOME_TIMEOUT_SEC {
                Err(format!(
                    "Timeout on waiting for final transaction outcome {}",
                    hash.to_string()
                ))?;
            }

            match response {
                Err(err) => match err.handler_error() {
                    Some(_err) => {
                        std::thread::sleep(time::Duration::from_secs(2));
                        continue;
                    }
                    _ => Err(format!("RpcTransactionError {}", err))?,
                },
                Ok(response) => return Ok(response),
            }
        }
    }

    fn call_change_method(
        &self,
        method_name: String,
        args: Vec<u8>,
        deposit: Option<Balance>,
        gas: Option<Gas>,
    ) -> Result<FinalExecutionOutcomeView, Box<dyn Error>> {
        self.call_change_method_batch(
            vec![method_name],
            vec![args],
            Some(vec![deposit.unwrap_or(0)]),
            gas,
        )
    }

    // fn wait_for_tx_final_outcome(
    //     hash: CryptoHash,
    //     account_id: AccountId,
    //     server_addr: url::Url,
    //     timeout_sec: u64,
    // ) -> Result<FinalExecutionOutcomeView, CustomError> {

    // }
}
