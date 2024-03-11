use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::error::Error;

pub struct NearRPCClient {
    endpoint_url: String,
    client: Client,
    api_key: String,
}

impl NearRPCClient {
    pub fn new(endpoint_url: &str, api_key: &Option<String>) -> Self {
        Self {
            endpoint_url: endpoint_url.to_string(),
            client: reqwest::blocking::Client::new(),
            api_key: api_key.clone().unwrap_or_default(),
        }
    }

    pub fn check_account_exists(&self, account_id: &str) -> Result<bool, Box<dyn Error>> {
        let json_value = json!({
            "id": "dontcare",
            "jsonrpc": "2.0",
            "method": "query",
            "params": {
                "request_type": "view_account",
                "finality": "final",
                "account_id": account_id,
            }
        });

        let res = self
            .client
            .post(&self.endpoint_url)
            .header("x-api-key", self.api_key.clone())
            .json(&json_value)
            .send()?
            .text()?;

        let val: Value = serde_json::from_str(&res)?;

        Ok(val["result"].is_object())
    }

    pub fn is_syncing(&self) -> Result<bool, Box<dyn Error>> {
        let json_value = json!({
            "id": "dontcare",
            "jsonrpc": "2.0",
            "method": "status",
            "params": []
        });

        let res = self
            .client
            .post(&self.endpoint_url)
            .header("x-api-key", self.api_key.clone())
            .json(&json_value)
            .send()?
            .text()?;

        let val: Value = serde_json::from_str(&res)?;

        if let Some(is_sync) = val["result"]["sync_info"]["syncing"].as_bool() {
            return Ok(is_sync);
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use crate::near_rpc_client::NearRPCClient;
    const NEAR_ENDPOINT: &str = "https://rpc.testnet.near.org";

    #[test]
    fn test_check_account_id() {
        let near_rpc_client = NearRPCClient::new(NEAR_ENDPOINT);
        assert!(near_rpc_client
            .check_account_exists("olga24912_3.testnet")
            .unwrap());

        assert!(!near_rpc_client
            .check_account_exists("dadadadasdsa.testnet")
            .unwrap());
    }

    #[test]
    fn test_is_syncing() {
        let rpc_client = NearRPCClient::new(NEAR_ENDPOINT);
        assert!(!rpc_client.is_syncing().unwrap());
    }
}
