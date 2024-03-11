use crate::dao_contract::DAOContract;
use crate::dao_types;
use crate::eth_client_contract::EthClientContract;
use crate::eth_client_contract_trait::EthClientContractTrait;
use eth2_utility::types::ClientMode;
use eth_types::eth2::{LightClientState, LightClientUpdate};
use eth_types::{BlockHeader, H256};
use near_primitives::views::FinalExecutionOutcomeView;
use std::error::Error;
use std::str::FromStr;
use std::thread;
use std::time::Duration;

/// Implementation of Ethereum Light Client Contract interaction on NEAR
/// having intermediate submission of Light Client Updates to the DAO contract.
pub struct DaoEthClientContract {
    /// Interface for an interaction with Ethereum Light Client Contract on NEAR
    eth_client_contract: EthClientContract,

    /// Interface for an interaction with DAO Contract
    dao_contract: DAOContract,
}

impl DaoEthClientContract {
    // Constructor for `DaoEthClientContract`
    pub fn new(eth_client_contract: EthClientContract, dao_contract: DAOContract) -> Self {
        Self {
            eth_client_contract,
            dao_contract,
        }
    }
}

impl EthClientContractTrait for DaoEthClientContract {
    fn send_light_client_update(
        &mut self,
        light_client_update: LightClientUpdate,
    ) -> Result<FinalExecutionOutcomeView, Box<dyn Error>> {
        // Submmit new proposal
        let (proposal_id, execution_outcome) =
            self.dao_contract.submit_light_client_update_proposal(
                near_sdk::AccountId::from_str(&self.eth_client_contract.get_account_id())?,
                light_client_update,
            )?;

        let max_num_of_iterations = 10;
        let mut count = 0;
        while count < max_num_of_iterations {
            let proposal_status = self.dao_contract.get_proposal(proposal_id);
            if let Ok(staus) = proposal_status {
                if staus.proposal.status != dao_types::ProposalStatus::InProgress {
                    break;
                }
            }

            thread::sleep(Duration::from_secs(10));
            count += 1;
        }

        Ok(execution_outcome)
    }

    fn is_ready_to_submit_light_client_update(&self) -> Result<bool, Box<dyn Error>> {
        // Check for already submitted updates
        let last_proposal_id = self.dao_contract.get_last_proposal_id()?;
        if last_proposal_id > 0 {
            let last_proposal_output = self.dao_contract.get_proposal(last_proposal_id - 1)?;
            if last_proposal_output.proposal.status == dao_types::ProposalStatus::InProgress
                && last_proposal_output.proposal.proposer.to_string()
                    == self
                        .dao_contract
                        .contract_wrapper
                        .get_signer_account_id()
                        .to_string()
            {
                return Ok(false);
            }
        }

        return Ok(true);
    }

    fn get_finalized_beacon_block_hash(&self) -> Result<H256, Box<dyn Error>> {
        self.eth_client_contract.get_finalized_beacon_block_hash()
    }

    fn get_finalized_beacon_block_slot(&self) -> Result<u64, Box<dyn Error>> {
        self.eth_client_contract.get_finalized_beacon_block_slot()
    }

    fn send_headers(
        &mut self,
        headers: &[BlockHeader],
    ) -> Result<FinalExecutionOutcomeView, Box<dyn std::error::Error>> {
        self.eth_client_contract.send_headers(headers)
    }

    fn get_client_mode(&self) -> Result<ClientMode, Box<dyn Error>> {
        self.eth_client_contract.get_client_mode()
    }

    fn get_light_client_state(&self) -> Result<LightClientState, Box<dyn Error>> {
        self.eth_client_contract.get_light_client_state()
    }

    fn get_last_block_number(&self) -> Result<u64, Box<dyn Error>> {
        self.eth_client_contract.get_last_block_number()
    }

    fn get_unfinalized_tail_block_number(&self) -> Result<Option<u64>, Box<dyn Error>> {
        self.eth_client_contract.get_unfinalized_tail_block_number()
    }
}

#[cfg(test)]
mod tests {
    use crate::eth_client_contract_trait::EthClientContractTrait;
    use crate::near_contract_wrapper::NearContractWrapper;
    use crate::{
        dao_contract, dao_eth_client_contract, eth_client_contract, eth_network,
        near_contract_wrapper, utils,
    };
    use eth_types::eth2::{ExtendedBeaconBlockHeader, LightClientUpdate, SyncCommittee};
    use eth_types::BlockHeader;
    use std::path::PathBuf;
    use tokio::runtime::Runtime;

    fn get_path(path: &str) -> PathBuf {
        let mut json_file_path = std::env::current_exe().unwrap();
        json_file_path.pop();
        json_file_path.push("../../../");
        json_file_path.push(path);

        json_file_path
    }

    #[ignore]
    #[test]
    fn test_smoke_dao_eth_client_contract_wrapper() {
        const PATH_TO_EXECUTION_BLOCKS: &str =
            "data/execution_block_headers_kiln_1099394-1099937.json";
        const PATH_TO_LIGHT_CLIENT_UPDATES: &str =
            "data/light_client_updates_kiln_1099394-1099937.json";
        const PATH_TO_CURRENT_SYNC_COMMITTEE: &str =
            "data/next_sync_committee_kiln_period_133.json";
        const PATH_TO_NEXT_LIGHT_CLIENT_UPDATE: &str =
            "data/next_sync_committee_kiln_period_134.json";

        let execution_blocks_json_file_path = get_path(PATH_TO_EXECUTION_BLOCKS);
        let light_client_update_json_file_path = get_path(PATH_TO_LIGHT_CLIENT_UPDATES);
        let current_sync_committee_path = get_path(PATH_TO_CURRENT_SYNC_COMMITTEE);
        let next_sync_committee_path = get_path(PATH_TO_NEXT_LIGHT_CLIENT_UPDATE);

        let rt = Runtime::new().unwrap();

        let worker = rt.block_on(workspaces::testnet()).unwrap();
        let signer = rt.block_on(worker.dev_create_account()).unwrap();
        let signer_private_key: String =
            utils::trim_quotes(serde_json::to_string(&signer.secret_key()).unwrap());
        let signer_account_id: String = format!("{}", signer.id());

        const NEAR_ENDPOINT: &str = "https://rpc.testnet.near.org";
        const CONTRACT_ACCOUNT_ID: &str = "dev-1660212590113-35162107482173";
        const DAO_CONTRACT_ACCOUNT_ID: &str = "eth2-test.sputnikv2.testnet";

        let near_contract_wrapper = Box::new(NearContractWrapper::new_with_raw_secret_key(
            NEAR_ENDPOINT,
            &signer_account_id,
            &signer_private_key,
            CONTRACT_ACCOUNT_ID,
            None,
            None,
        ));

        let eth_client = eth_client_contract::EthClientContract::new(near_contract_wrapper);

        let execution_blocks: Vec<BlockHeader> = serde_json::from_str(
            &std::fs::read_to_string(execution_blocks_json_file_path).expect("Unable to read file"),
        )
        .unwrap();

        let light_client_updates: Vec<LightClientUpdate> = serde_json::from_str(
            &std::fs::read_to_string(light_client_update_json_file_path)
                .expect("Unable to read file"),
        )
        .unwrap();

        let current_sync_committee: SyncCommittee = serde_json::from_str(
            &std::fs::read_to_string(current_sync_committee_path).expect("Unable to read file"),
        )
        .unwrap();
        let next_sync_committee: SyncCommittee = serde_json::from_str(
            &std::fs::read_to_string(next_sync_committee_path).expect("Unable to read file"),
        )
        .unwrap();

        let finalized_beacon_header = ExtendedBeaconBlockHeader::from(
            light_client_updates[0]
                .clone()
                .finality_update
                .header_update,
        );

        let finalized_hash = light_client_updates[0]
            .clone()
            .finality_update
            .header_update
            .execution_block_hash;
        let mut finalized_execution_header = None::<BlockHeader>;
        for header in &execution_blocks {
            if header.hash.unwrap() == finalized_hash {
                finalized_execution_header = Some(header.clone());
                break;
            }
        }

        eth_client.init_contract(
            eth_network::EthNetwork::Kiln,
            finalized_execution_header.unwrap(),
            finalized_beacon_header,
            current_sync_committee,
            next_sync_committee,
            Some(true),
            Some(false),
            None,
            Some(eth_client.contract_wrapper.get_signer_account_id()),
        );

        let dao_contract_wrapper =
            near_contract_wrapper::NearContractWrapper::new_with_raw_secret_key(
                NEAR_ENDPOINT,
                &signer_account_id,
                &signer_private_key,
                DAO_CONTRACT_ACCOUNT_ID,
                None,
            );
        let dao_contract = dao_contract::DAOContract::new(Box::new(dao_contract_wrapper));
        let mut dao_client =
            dao_eth_client_contract::DaoEthClientContract::new(eth_client, dao_contract);

        let finalized_slot = dao_client.get_finalized_beacon_block_slot().unwrap();
        let mut next_light_client_update = None;
        for i in 0..light_client_updates.len() {
            if light_client_updates[i]
                .finality_update
                .header_update
                .beacon_header
                .slot
                == finalized_slot
            {
                next_light_client_update = Some(light_client_updates[i + 1].clone());
            }
        }

        for block in &execution_blocks {
            dao_client.send_headers(&vec![block.clone()]).unwrap();

            if block.hash.unwrap()
                == next_light_client_update
                    .clone()
                    .unwrap()
                    .finality_update
                    .header_update
                    .execution_block_hash
            {
                dao_client
                    .send_light_client_update(next_light_client_update.unwrap())
                    .unwrap();
                break;
            }
        }
    }
}
