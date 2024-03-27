use crate::types::{BlockHeader, TransactionReceipt};
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::error::Error;
use ethereum_types::{H256, U64};

pub struct EthRPCClient {
    endpoint_url: String,
    client: Client,
}

impl EthRPCClient {
    pub fn new(endpoint_url: &str) -> Self {
        Self {
            endpoint_url: endpoint_url.to_string(),
            client: reqwest::blocking::Client::new(),
        }
    }

    pub fn get_transaction_receipt_by_hash(&self, tx_hash: &H256) -> Result<TransactionReceipt, Box<dyn Error>> {
        let json_value = json!({
            "id": 1,
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash.0]
        });

        let res = self
            .client
            .post(&self.endpoint_url)
            .json(&json_value)
            .send()?
            .text()?;

        let val: Value = serde_json::from_str(&res)?;
        let receipt = TransactionReceipt::deserialize(&val["result"])?;

        Ok(receipt)
    }

    pub fn get_block_by_number(&self, block_number: U64) -> Result<BlockHeader, Box<dyn Error>> {
        let json_value = json!({
            "id": 1,
            "jsonrpc": "2.0",
            "method": "eth_getBlockByNumber",
            "params": [format!("0x{:x}", block_number), false]
        });

        let res = self
            .client
            .post(&self.endpoint_url)
            .json(&json_value)
            .send()?
            .text()?;

        let val: Value = serde_json::from_str(&res)?;
        let header = BlockHeader::deserialize(&val["result"])?;

        Ok(header)
    }

    pub fn get_block_receipts(
        &self,
        block_number: U64,
    ) -> Result<Vec<TransactionReceipt>, Box<dyn Error>> {
        let json_value = json!({
            "id": 1,
            "jsonrpc": "2.0",
            "method": "eth_getBlockReceipts",
            "params": [format!("0x{:x}", block_number)]
        });

        let res = self
            .client
            .post(&self.endpoint_url)
            .json(&json_value)
            .send()?
            .text()?;

        let val: Value = serde_json::from_str(&res)?;
        let receipts = Vec::<TransactionReceipt>::deserialize(&val["result"])?;

        Ok(receipts)
    }
}
