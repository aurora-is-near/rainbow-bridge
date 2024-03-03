use eth_types::*;
use near_plugins::{
    access_control, access_control_any, pause, AccessControlRole, AccessControllable, Pausable,
    Upgradable,
};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, ext_contract, near_bindgen, Gas, PanicOnDefault, Promise, PromiseOrValue, PublicKey,
};
use rlp::Rlp;

type AccountId = String;

/// Gas to call block_hash_safe
const BLOCK_HASH_SAFE_GAS: Gas = Gas(10_000_000_000_000);

/// Gas to call on_block_hash
const ON_BLOCK_HASH_GAS: Gas = Gas(5_000_000_000_000);

#[derive(AccessControlRole, Deserialize, Serialize, Copy, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum Role {
    PauseManager,
    UnrestrictedVerifyLogEntry,
    UnrestrictedVerifyStorageProof,
    UpgradableCodeStager,
    UpgradableCodeDeployer,
    DAO,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault, Pausable, Upgradable)]
#[access_control(role_type(Role))]
#[pausable(manager_roles(Role::PauseManager, Role::DAO))]
#[upgradable(access_control_roles(
    code_stagers(Role::UpgradableCodeStager, Role::DAO),
    code_deployers(Role::UpgradableCodeDeployer, Role::DAO),
    duration_initializers(Role::DAO),
    duration_update_stagers(Role::DAO),
    duration_update_appliers(Role::DAO),
))]
pub struct EthProver {
    bridge_smart_contract: AccountId,
    #[deprecated]
    paused: u128,
}

/// Defines an interface to call EthProver back as a callback with the result from the
/// EthClient contract.
#[ext_contract(remote_self)]
pub trait RemoteSelf {
    #[result_serializer(borsh)]
    fn on_block_hash(&self, #[serializer(borsh)] expected_block_hash: H256) -> bool;
}

/// Defines an interface to call EthClient contract to get the safe block hash for a given block
/// number. It returns Some(hash) if the block hash is present in the safe canonical chain, or
/// None if the block number is not part of the canonical chain yet.
#[ext_contract(eth_client)]
pub trait RemoteEthClient {
    #[result_serializer(borsh)]
    fn block_hash_safe(&self, #[serializer(borsh)] index: u64) -> Option<H256>;
}

/// Get element at position `pos` from rlp encoded data,
/// and decode it as vector of bytes
fn get_vec(data: &Rlp, pos: usize) -> Vec<u8> {
    data.at(pos).unwrap().as_val::<Vec<u8>>().unwrap()
}

#[near_bindgen]
impl EthProver {
    #[init]
    #[private]
    pub fn init(#[serializer(borsh)] bridge_smart_contract: AccountId) -> Self {
        #[allow(deprecated)]
        let mut contract = Self {
            bridge_smart_contract,
            paused: 0,
        };

        contract.acl_init_super_admin(near_sdk::env::predecessor_account_id());
        contract
    }

    /// Implementation of the callback when the EthClient returns data.
    /// This method can only be called by the EthProver contract itself (e.g. as callback).
    /// - `block_hash` is the actual data from the EthClient call
    /// - `expected_block_hash` is the block hash that we expect to be passed by us.
    #[result_serializer(borsh)]
    #[private]
    pub fn on_block_hash(
        &self,
        #[callback]
        #[serializer(borsh)]
        block_hash: Option<H256>,
        #[serializer(borsh)] expected_block_hash: H256,
    ) -> bool {
        return block_hash == Some(expected_block_hash);
    }

    /// Externally visible method to verify that the given block hash is part of the safe canonical
    /// chain on the remote EthClient contract.
    /// Returns a promise.
    #[result_serializer(borsh)]
    pub fn assert_ethclient_hash(
        &self,
        #[serializer(borsh)] block_number: u64,
        #[serializer(borsh)] expected_block_hash: H256,
    ) -> PromiseOrValue<bool> {
        eth_client::ext(self.bridge_smart_contract.parse().unwrap())
            .with_static_gas(BLOCK_HASH_SAFE_GAS)
            .block_hash_safe(block_number)
            .then(
                remote_self::ext(env::current_account_id())
                    .with_static_gas(ON_BLOCK_HASH_GAS)
                    .on_block_hash(expected_block_hash),
            )
            .into()
    }

    #[pause(except(roles(Role::UnrestrictedVerifyLogEntry, Role::DAO)))]
    #[result_serializer(borsh)]
    pub fn verify_log_entry(
        &self,
        #[serializer(borsh)] log_index: u64,
        #[serializer(borsh)] log_entry_data: Vec<u8>,
        #[serializer(borsh)] receipt_index: u64,
        #[serializer(borsh)] receipt_data: Vec<u8>,
        #[serializer(borsh)] header_data: Vec<u8>,
        #[serializer(borsh)] proof: Vec<Vec<u8>>,
        #[serializer(borsh)] skip_bridge_call: bool,
    ) -> PromiseOrValue<bool> {
        let min_header_height = None;
        let max_header_height = None;
        self.verify_log_entry_internal(
            log_index,
            log_entry_data,
            receipt_index,
            receipt_data,
            header_data,
            proof,
            min_header_height,
            max_header_height,
            skip_bridge_call,
        )
    }

    #[pause(except(roles(Role::UnrestrictedVerifyLogEntry, Role::DAO)))]
    #[result_serializer(borsh)]
    pub fn verify_log_entry_in_bound(
        &self,
        #[serializer(borsh)] log_index: u64,
        #[serializer(borsh)] log_entry_data: Vec<u8>,
        #[serializer(borsh)] receipt_index: u64,
        #[serializer(borsh)] receipt_data: Vec<u8>,
        #[serializer(borsh)] header_data: Vec<u8>,
        #[serializer(borsh)] proof: Vec<Vec<u8>>,
        #[serializer(borsh)] min_header_height: Option<u64>,
        #[serializer(borsh)] max_header_height: Option<u64>,
        #[serializer(borsh)] skip_bridge_call: bool,
    ) -> PromiseOrValue<bool> {
        self.verify_log_entry_internal(
            log_index,
            log_entry_data,
            receipt_index,
            receipt_data,
            header_data,
            proof,
            min_header_height,
            max_header_height,
            skip_bridge_call,
        )
    }

    fn verify_log_entry_internal(
        &self,
        #[serializer(borsh)] log_index: u64,
        #[serializer(borsh)] log_entry_data: Vec<u8>,
        #[serializer(borsh)] receipt_index: u64,
        #[serializer(borsh)] receipt_data: Vec<u8>,
        #[serializer(borsh)] header_data: Vec<u8>,
        #[serializer(borsh)] proof: Vec<Vec<u8>>,
        #[serializer(borsh)] min_header_height: Option<u64>,
        #[serializer(borsh)] max_header_height: Option<u64>,
        #[serializer(borsh)] skip_bridge_call: bool,
    ) -> PromiseOrValue<bool> {
        let header: BlockHeader = rlp::decode(header_data.as_slice()).unwrap();
        if !Self::is_block_height_in_bound(header.number, min_header_height, max_header_height) {
            return PromiseOrValue::Value(false);
        }

        let log_entry: LogEntry = rlp::decode(log_entry_data.as_slice()).unwrap();
        let receipt: Receipt = rlp::decode(receipt_data.as_slice()).unwrap();

        // Verify log_entry included in receipt
        let log_index_usize = usize::try_from(log_index).expect("Invalid log_index");
        assert_eq!(receipt.logs[log_index_usize], log_entry);

        // Verify receipt included into header
        let data = Self::verify_trie_proof(
            header.receipts_root,
            rlp::encode(&receipt_index).to_vec(),
            proof,
        );
        let verification_result = receipt_data == data;
        if verification_result && skip_bridge_call {
            return PromiseOrValue::Value(true);
        } else if !verification_result {
            return PromiseOrValue::Value(false);
        }

        // Verify block header was in the bridge
        eth_client::ext(self.bridge_smart_contract.parse().unwrap())
            .with_static_gas(BLOCK_HASH_SAFE_GAS)
            .block_hash_safe(header.number)
            .then(
                remote_self::ext(env::current_account_id())
                    .with_static_gas(ON_BLOCK_HASH_GAS)
                    .on_block_hash(header.hash.unwrap()),
            )
            .into()
    }

    /// WARNING: When the value is not found, `eth_getProof` will return "0x0" at
    /// the StorageProof `value` field.  In order to verify the proof of non
    /// existence, you must set `value` to empty vec, *not* the RLP encoding of 0 or null
    /// (which would be 0x80).
    #[pause(except(roles(Role::UnrestrictedVerifyStorageProof, Role::DAO)))]
    #[result_serializer(borsh)]
    pub fn verify_storage_proof(
        &self,
        #[serializer(borsh)] header_data: Vec<u8>,
        #[serializer(borsh)] account_proof: Vec<Vec<u8>>, // account proof
        #[serializer(borsh)] contract_address: Vec<u8>,   // eth address
        #[serializer(borsh)] expected_account_state: Vec<u8>, // encoded account state
        #[serializer(borsh)] storage_key_hash: Vec<u8>,   // keccak256 of storage key
        #[serializer(borsh)] storage_proof: Vec<Vec<u8>>, // storage proof
        #[serializer(borsh)] expected_storage_value: Vec<u8>, // storage value
        #[serializer(borsh)] min_header_height: Option<u64>,
        #[serializer(borsh)] max_header_height: Option<u64>,
        #[serializer(borsh)] skip_bridge_call: bool,
    ) -> PromiseOrValue<bool> {
        let header: BlockHeader = rlp::decode(header_data.as_slice()).unwrap();
        if !Self::is_block_height_in_bound(header.number, min_header_height, max_header_height) {
            return PromiseOrValue::Value(false);
        }

        let account_key = near_keccak256(&contract_address).to_vec();
        let account_state = Self::verify_trie_proof(header.state_root, account_key, account_proof);
        if account_state != expected_account_state {
            env::log_str("account_state != expected_account_state");
            return PromiseOrValue::Value(false);
        }

        let storage_hash: H256 = Rlp::new(&account_state).val_at(2).unwrap();
        let storage_value = Self::verify_trie_proof(storage_hash, storage_key_hash, storage_proof);
        if storage_value != expected_storage_value {
            env::log_str("storage_value != expected_storage_value");
            return PromiseOrValue::Value(false);
        }

        if skip_bridge_call {
            return PromiseOrValue::Value(true);
        }

        // Verify block header was in the bridge
        eth_client::ext(self.bridge_smart_contract.parse().unwrap())
            .with_static_gas(BLOCK_HASH_SAFE_GAS)
            .block_hash_safe(header.number)
            .then(
                remote_self::ext(env::current_account_id())
                    .with_static_gas(ON_BLOCK_HASH_GAS)
                    .on_block_hash(header.hash.unwrap()),
            )
            .into()
    }

    fn is_block_height_in_bound(
        header_height: u64,
        min_header_height: Option<u64>,
        max_header_height: Option<u64>,
    ) -> bool {
        if let Some(min_header_height) = min_header_height {
            if header_height < min_header_height {
                env::log_str(
                    format!(
                        "Block height {} < Minimum header height {}",
                        header_height, min_header_height
                    )
                    .as_str(),
                );
                return false;
            }
        }

        if let Some(max_header_height) = max_header_height {
            if header_height > max_header_height {
                env::log_str(
                    format!(
                        "Block height {} > Maximum header height {}",
                        header_height, max_header_height
                    )
                    .as_str(),
                );
                return false;
            }
        }

        true
    }

    /// Verify the proof recursively traversing through the key.
    /// Return the value at the end of the key, in case the proof is valid.
    ///
    /// @param expected_root is the expected root of the current node.
    /// @param key is the key for which we are proving the value.
    /// @param proof contains relevant information to verify data is valid
    ///
    /// Patricia Trie: https://eth.wiki/en/fundamentals/patricia-tree
    /// Patricia Img:  https://ethereum.stackexchange.com/questions/268/ethereum-block-architecture/6413#6413
    ///
    /// Verification:  https://github.com/slockit/in3/wiki/Ethereum-Verification-and-MerkleProof#receipt-proof
    /// Article:       https://medium.com/@ouvrard.pierre.alain/merkle-proof-verification-for-ethereum-patricia-tree-48f29658eec
    /// Python impl:   https://gist.github.com/mfornet/0ff283274c0162f1cca45966bccf69ee
    ///
    fn verify_trie_proof(expected_root: H256, key: Vec<u8>, proof: Vec<Vec<u8>>) -> Vec<u8> {
        let mut actual_key = vec![];
        for el in key {
            actual_key.push(el / 16);
            actual_key.push(el % 16);
        }
        Self::_verify_trie_proof((expected_root.0).0.into(), &actual_key, &proof, 0, 0)
    }

    fn _verify_trie_proof(
        expected_root: Vec<u8>,
        key: &Vec<u8>,
        proof: &Vec<Vec<u8>>,
        key_index: usize,
        proof_index: usize,
    ) -> Vec<u8> {
        let node = &proof[proof_index];

        if key_index == 0 {
            // trie root is always a hash
            assert_eq!(near_keccak256(node), expected_root.as_slice());
        } else if node.len() < 32 {
            // if rlp < 32 bytes, then it is not hashed
            assert_eq!(node.as_slice(), expected_root);
        } else {
            assert_eq!(near_keccak256(node), expected_root.as_slice());
        }

        let node = Rlp::new(&node.as_slice());

        if node.iter().count() == 17 {
            // Branch node
            if key_index >= key.len() {
                assert_eq!(proof_index + 1, proof.len());
                get_vec(&node, 16)
            } else {
                let new_expected_root = get_vec(&node, key[key_index] as usize);
                if !new_expected_root.is_empty() {
                    Self::_verify_trie_proof(
                        new_expected_root,
                        key,
                        proof,
                        key_index + 1,
                        proof_index + 1,
                    )
                } else {
                    // not included in proof
                    vec![]
                }
            }
        } else {
            // Leaf or extension node
            assert_eq!(node.iter().count(), 2);
            let path_u8 = get_vec(&node, 0);
            // Extract first nibble
            let head = path_u8[0] / 16;
            // assert!(0 <= head); is implicit because of type limits
            assert!(head <= 3);

            // Extract path
            let mut path = vec![];
            if head % 2 == 1 {
                path.push(path_u8[0] % 16);
            }
            for val in path_u8.iter().skip(1) {
                path.push(val / 16);
                path.push(val % 16);
            }

            if head >= 2 {
                // Leaf node
                assert_eq!(proof_index + 1, proof.len());
                assert_eq!(key_index + path.len(), key.len());
                if path.as_slice() == &key[key_index..key_index + path.len()] {
                    get_vec(&node, 1)
                } else {
                    vec![]
                }
            } else {
                // Extension node
                if key_index + path.len() <= key.len() &&
                    path.as_slice() == &key[key_index..key_index + path.len()] {
                    let new_expected_root = get_vec(&node, 1);
                    Self::_verify_trie_proof(
                        new_expected_root,
                        key,
                        proof,
                        key_index + path.len(),
                        proof_index + 1,
                    )
                } else {
                    vec![]
                }
            }
        }
    }

    #[access_control_any(roles(Role::DAO))]
    pub fn set_bridge(&mut self, bridge: AccountId) {
        env::log_str(
            format!(
                "Old bridge account: {} New bridge account {}",
                self.bridge_smart_contract, bridge
            )
            .as_str(),
        );
        self.bridge_smart_contract = bridge;
    }

    #[access_control_any(roles(Role::DAO))]
    pub fn attach_full_access_key(&self, public_key: PublicKey) -> Promise {
        Promise::new(env::current_account_id()).add_full_access_key(public_key)
    }

    pub fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_owned()
    }
}

#[cfg(test)]
mod tests;
mod tests_storage_proof;
