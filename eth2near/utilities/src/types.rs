use crate::primitives::{Bytes, U8};
use ethereum_types::{H256, Address, Bloom, U64, U128};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockHeader {
    pub parent_hash: H256,
    pub sha3_uncles: H256,
    pub miner: Address,
    pub state_root: H256,
    pub transactions_root: H256,
    pub receipts_root: H256,
    pub logs_bloom: Bloom,
    pub difficulty: U128,
    pub number: Bytes,
    pub gas_limit: Bytes,
    pub gas_used: Bytes,
    pub timestamp: Bytes,
    pub extra_data: Bytes,
    pub mix_hash: H256,
    pub nonce: Bytes,
    pub base_fee_per_gas: Option<U64>,
    pub withdrawals_root: Option<H256>,
    pub blob_gas_used: Option<U64>,
    pub excess_blob_gas: Option<U64>,
    pub parent_beacon_block_root: Option<H256>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Log {
    pub address: Address,
    pub topics: Vec<H256>,
    pub data: Bytes,
    pub log_index: U64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionReceipt {
    pub block_number: U64,
    pub transaction_index: U64,
    #[serde(rename = "type")]
    pub transaction_type: U8,
    pub cumulative_gas_used: Bytes,
    pub logs_bloom: Bloom,
    pub logs: Vec<Log>,
    pub status: U8,
}
