use crate::{
    eth_rpc_client::EthRPCClient,
    types::{BlockHeader, Log, TransactionReceipt},
    primitives::U8
};
use cita_trie::{MemoryDB, PatriciaTrie, Trie, TrieError};
use hasher::HasherKeccak;
use rlp::RlpStream;
use std::error::Error;
use std::sync::Arc;
use ethereum_types::{H256, U64};

#[derive(Debug)]
pub struct Proof {
    pub log_index: U64,
    pub log_entry_data: Vec<u8>,
    pub receipt_index: U64,
    pub receipt_data: Vec<u8>,
    pub header_data: Vec<u8>,
    pub proof: Vec<Vec<u8>>,
}

pub fn get_proof_for_event(tx_hash: H256, log_index: u64, node_url: &str) -> Result<Proof, Box<dyn Error>> {
    let client = EthRPCClient::new(node_url);

    let receipt = client.get_transaction_receipt_by_hash(&tx_hash)?;
    let block_header = client.get_block_by_number(receipt.block_number)?;
    let block_receipts = client.get_block_receipts(receipt.block_number)?;

    let mut trie = build_receipt_trie(&block_receipts)?;
    trie.root()?;

    let receipt_key = rlp::encode(&receipt.transaction_index);
    let proof = trie.get_proof(&receipt_key)?;

    let mut log_data: Option<Vec<u8>> = None;
    let mut log_index_in_receipt = 0;
    for (i, log) in receipt.logs.iter().enumerate() {
        if log.log_index == log_index.into() {
            log_data = Some(encode_log(log));
            log_index_in_receipt = i;
        }
    }

    Ok(Proof {
        log_index: log_index_in_receipt.into(),
        log_entry_data: log_data.ok_or("Log not found")?,
        receipt_index: receipt.transaction_index,
        receipt_data: encode_receipt(&receipt),
        header_data: encode_header(&block_header),
        proof,
    })
}

fn build_receipt_trie(receipts: &[TransactionReceipt],) -> Result<PatriciaTrie<MemoryDB, HasherKeccak>, TrieError> {
    let memdb = Arc::new(MemoryDB::new(true));
    let hasher = Arc::new(HasherKeccak::new());
    let mut trie = PatriciaTrie::new(memdb, hasher);

    for receipt in receipts {
        let receipt_key = rlp::encode(&receipt.transaction_index).to_vec();
        let receipt_data = encode_receipt(receipt);

        trie.insert(receipt_key, receipt_data)?;
    }

    Ok(trie)
}

fn encode_receipt(receipt: &TransactionReceipt) -> Vec<u8> {
    let mut stream = RlpStream::new();

    if receipt.transaction_type != U8(0) {
        stream.append(&receipt.transaction_type);
    }

    stream.begin_list(4);
    stream
        .append(&receipt.status)
        .append(&receipt.cumulative_gas_used)
        .append(&receipt.logs_bloom);

    stream.begin_list(receipt.logs.len());
    for log in &receipt.logs {
        stream.begin_list(3);
        stream.append(&log.address);

        stream.begin_list(log.topics.len());
        for topic in &log.topics {
            stream.append(topic);
        }

        stream.append(&log.data);
    }

    stream.out().to_vec()
}

fn encode_log(log: &Log) -> Vec<u8> {
    let mut stream = RlpStream::new();
    stream.begin_list(3);

    stream.append(&log.address);

    stream.begin_list(log.topics.len());
    for topic in &log.topics {
        stream.append(topic);
    }

    stream.append(&log.data);

    stream.out().to_vec()
}

fn encode_header(header: &BlockHeader) -> Vec<u8> {
    let mut stream = RlpStream::new();
    stream.begin_unbounded_list();

    stream
        .append(&header.parent_hash)
        .append(&header.sha3_uncles)
        .append(&header.miner)
        .append(&header.state_root)
        .append(&header.transactions_root)
        .append(&header.receipts_root)
        .append(&header.logs_bloom)
        .append(&header.difficulty)
        .append(&header.number)
        .append(&header.gas_limit)
        .append(&header.gas_used)
        .append(&header.timestamp)
        .append(&header.extra_data)
        .append(&header.mix_hash)
        .append(&header.nonce);

    header.base_fee_per_gas.map(|v| stream.append(&v));
    header.withdrawals_root
        .as_ref()
        .map(|v| stream.append(v));
    header.blob_gas_used.map(|v| stream.append(&v));
    header.excess_blob_gas.map(|v| stream.append(&v));
    header.parent_beacon_block_root
        .as_ref()
        .map(|v| stream.append(v));

    stream.finalize_unbounded_list();
    stream.out().to_vec()
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::{fs, str::FromStr};
    use hasher::Hasher;
    use serde_json::Value;
    use std::path::PathBuf;
    
    const RPC_URL: &str = "https://eth.llamarpc.com";

    /*
     * Test data format:
     * log_index - index of the log within transaction receipt (can be obtained from ETH RPC)
     * receipt_index - index of the transaction receipt within the block (can be obtained from ETH RPC)
     * block_hash - hash of the block containing the transaction
     * receipt - RLP encoded transaction receipt (can be generated using other libraries, like eth-object.js)
     * log - RLP encoded log entry (can be generated using other libraries, like eth-object.js)
     * proof - merkle proof that receipt is part of the block receipt trie. To get the proof, first create a Merkle-Patricia tree including
     *   all RLP encoded transaction receipts of the block. The root of the tree must be the same as the receiptsRoot field of the block header.
     *   Then calculate merkle proof. One can use merkle-patricia-tree.js to build and generate the proof for the tree.
     */

    #[test]
    fn generate_proof_pre_shapella() {
        let tx_hash = H256::from_str("0xc4a6c5cde1d243b26b013f805f71f6de91536f66c993abfee746f373203b68cc").unwrap();
        let proof = get_proof_for_event(tx_hash, 251, RPC_URL).unwrap();
        verify_proof(proof, "pre_shapella_proof.json");
    }

    #[test]
    fn generate_proof_post_shapella() {
        let tx_hash = H256::from_str("0xd6ae351d6946f98c4b63589e2154db668e703e8c09fbd4e5c6807b5d356453c3").unwrap();
        let proof = get_proof_for_event(tx_hash, 172, RPC_URL).unwrap();
        verify_proof(proof, "post_shapella_proof.json");
    }
    
    #[test]
    fn generate_proof_post_dencun() {
        let tx_hash = H256::from_str("0x42639810a1238a76ca947b848f5b88a854ac36471d1c4f6a15631393790f89af").unwrap();
        let proof = get_proof_for_event(tx_hash, 360, RPC_URL).unwrap();
        verify_proof(proof, "post_dencun_proof.json");
    }

    fn read_proof_data(file_name: &str) -> (u64, u64, String, String, String, Vec<String>) {
        let mut data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        data_dir.push("src/test_data");
        data_dir.push(file_name);

        let data = fs::read_to_string(data_dir).unwrap();
        let obj: Value = serde_json::from_str(&data).unwrap();

        let expected_log_index = obj["log_index"].as_u64().unwrap();
        let expected_receipt_index = obj["receipt_index"].as_u64().unwrap();
        let expected_header = obj["block_hash"].as_str().unwrap().into();
        let expected_receipt = obj["receipt"].as_str().unwrap().into();
        let expected_log = obj["log"].as_str().unwrap().into();
        let expected_proof = obj["proof"].as_array().unwrap()
            .iter().map(|x| x.as_str().unwrap().into()).collect::<Vec<String>>();

        (expected_log_index, expected_receipt_index, expected_header, expected_receipt, expected_log, expected_proof)
    }

    fn verify_proof(proof: Proof, test_file: &str) {
        let (expected_log_index, expected_receipt_index, expected_header, expected_receipt, expected_log, expected_proof)
            = read_proof_data(test_file);

        let hasher = HasherKeccak::new();
        assert_eq!(hasher.digest(&proof.header_data), hex::decode(expected_header).unwrap());

        assert_eq!(proof.log_index, expected_log_index.into());
        assert_eq!(proof.receipt_index, expected_receipt_index.into());
        assert_eq!(proof.receipt_data, hex::decode(expected_receipt).unwrap());
        assert_eq!(proof.log_entry_data, hex::decode(expected_log).unwrap());
        assert_eq!(proof.proof.len(), expected_proof.len());
        assert!(proof.proof.into_iter().eq(expected_proof.iter().map(|x| hex::decode(x).unwrap())));
    }
}
