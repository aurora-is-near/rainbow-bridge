use crate::config::Config;
use crate::prometheus_metrics;
use crate::prometheus_metrics::{
    CHAIN_FINALIZED_EXECUTION_BLOCK_HEIGHT_ON_ETH, CHAIN_FINALIZED_EXECUTION_BLOCK_HEIGHT_ON_NEAR,
    FAILS_ON_HEADERS_SUBMISSION, FAILS_ON_UPDATES_SUBMISSION,
    LAST_FINALIZED_ETH_SLOT, LAST_FINALIZED_ETH_SLOT_ON_NEAR,
};
use bitvec::macros::internal::funty::Fundamental;
use contract_wrapper::eth_client_contract_trait::EthClientContractTrait;
use contract_wrapper::near_rpc_client::NearRPCClient;
use eth2_utility::consensus::{EPOCHS_PER_SYNC_COMMITTEE_PERIOD, SLOTS_PER_EPOCH};
use eth_rpc_client::beacon_rpc_client::BeaconRPCClient;
use eth_rpc_client::errors::NoBlockForSlotError;
use eth_rpc_client::eth1_rpc_client::Eth1RPCClient;
use eth_rpc_client::hand_made_finality_light_client_update::HandMadeFinalityLightClientUpdate;
use eth_types::eth2::LightClientUpdate;
use eth_types::BlockHeader;
use log::{debug, info, trace, warn};
use near_primitives::views::FinalExecutionStatus;
use std::{cmp, fmt};
use std::cmp::max;
use std::error::Error;
use std::fmt::Display;
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use std::vec::Vec;
use eth2_utility::types::ClientMode;
use types::Slot;

const ONE_EPOCH_IN_SLOTS: u64 = 32;

macro_rules! skip_fail {
    ($res:expr, $msg:expr, $sleep_time:expr) => {
        match $res {
            Ok(val) => val,
            Err(e) => {
                warn!(target: "relay", "{}. Error: {}", $msg, e);
                trace!(target: "relay", "Sleep {} secs before next loop", $sleep_time);
                thread::sleep(Duration::from_secs($sleep_time));
                continue;
            }
        }
    };
}

macro_rules! return_on_fail {
    ($res:expr, $msg:expr) => {
        match $res {
            Ok(val) => val,
            Err(e) => {
                warn!(target: "relay", "{}. Error: {}", $msg, e);
                return;
            }
        }
    };
}

macro_rules! return_val_on_fail {
    ($res:expr, $msg:expr, $val:expr) => {
        match $res {
            Ok(val) => val,
            Err(e) => {
                warn!(target: "relay", "{}. Error: {}", $msg, e);
                return $val;
            }
        }
    };
}

macro_rules! return_val_on_fail_and_sleep {
    ($res:expr, $msg:expr, $sleep_time:expr, $val:expr) => {
        match $res {
            Ok(val) => val,
            Err(e) => {
                warn!(target: "relay", "{}. Error: {}", $msg, e);
                trace!(target: "relay", "Sleep {} secs before next loop", $sleep_time);
                thread::sleep(Duration::from_secs($sleep_time));
                return $val;
            }
        }
    };
}

#[derive(Debug)]
pub struct SlotByBlockNumberNotFound;

impl Display for SlotByBlockNumberNotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Slot wasn't found for given block number"
        )
    }
}

impl Error for SlotByBlockNumberNotFound {}

pub struct Eth2NearRelay {
    beacon_rpc_client: BeaconRPCClient,
    eth1_rpc_client: Eth1RPCClient,
    near_rpc_client: NearRPCClient,
    eth_client_contract: Box<dyn EthClientContractTrait>,
    headers_batch_size: u64,
    ethereum_network: String,
    interval_between_light_client_updates_submission_in_epochs: u64,
    max_blocks_for_finalization: u64,
    near_network_name: String,
    terminate: bool,
    next_light_client_update: Option<LightClientUpdate>,
    sleep_time_on_sync_secs: u64,
    sleep_time_after_submission_secs: u64,
    get_light_client_update_by_epoch: bool,
}

impl Eth2NearRelay {
    pub fn init(
        config: &Config,
        eth_contract: Box<dyn EthClientContractTrait>,
    ) -> Self {
        info!(target: "relay", "=== Relay initialization === ");

        let beacon_rpc_client = BeaconRPCClient::new(
            &config.beacon_endpoint,
            config.eth_requests_timeout_seconds,
            config.state_requests_timeout_seconds,
            Some(config.beacon_rpc_version.clone()),
        );
        let next_light_client_update =
            Self::get_light_client_update_from_file(config, &beacon_rpc_client)
                .expect("Error on parsing light client update");

        let eth2near_relay = Eth2NearRelay {
            beacon_rpc_client,
            eth1_rpc_client: Eth1RPCClient::new(&config.eth1_endpoint),
            eth_client_contract: eth_contract,
            near_rpc_client: NearRPCClient::new(&config.near_endpoint),
            headers_batch_size: config.headers_batch_size as u64,
            ethereum_network: config.ethereum_network.to_string(),
            interval_between_light_client_updates_submission_in_epochs: config
                .interval_between_light_client_updates_submission_in_epochs,
            max_blocks_for_finalization: config.max_blocks_for_finalization,
            near_network_name: config.near_network_id.to_string(),
            terminate: false,
            next_light_client_update,
            sleep_time_on_sync_secs: config.sleep_time_on_sync_secs,
            sleep_time_after_submission_secs: config.sleep_time_after_submission_secs,
            get_light_client_update_by_epoch: config
                .get_light_client_update_by_epoch
                .unwrap_or(false),
        };

        if let Some(port) = config.prometheus_metrics_port {
            thread::spawn(move || prometheus_metrics::run_prometheus_service(port));
        }

        eth2near_relay
    }

    fn get_last_finalized_slot_on_near(&self) -> Result<u64, Box<dyn Error>> {
        let last_finalized_slot_on_near =
            self.eth_client_contract.get_finalized_beacon_block_slot()?;
        LAST_FINALIZED_ETH_SLOT_ON_NEAR.inc_by(cmp::max(
            0,
            last_finalized_slot_on_near as i64 - LAST_FINALIZED_ETH_SLOT_ON_NEAR.get(),
        ));

        if let Ok(last_block_number) = self
            .beacon_rpc_client
            .get_block_number_for_slot(Slot::new(last_finalized_slot_on_near))
        {
            CHAIN_FINALIZED_EXECUTION_BLOCK_HEIGHT_ON_NEAR.inc_by(cmp::max(
                0,
                last_block_number as i64 - CHAIN_FINALIZED_EXECUTION_BLOCK_HEIGHT_ON_NEAR.get(),
            ));
        }

        Ok(last_finalized_slot_on_near)
    }

    fn get_last_finalized_slot_on_eth(&self) -> Result<u64, Box<dyn Error>> {
        let last_finalized_slot_on_eth = self
            .beacon_rpc_client
            .get_last_finalized_slot_number()?
            .as_u64();

        LAST_FINALIZED_ETH_SLOT.inc_by(cmp::max(
            0,
            last_finalized_slot_on_eth as i64 - LAST_FINALIZED_ETH_SLOT.get(),
        ));

        if let Ok(last_block_number) = self
            .beacon_rpc_client
            .get_block_number_for_slot(Slot::new(last_finalized_slot_on_eth))
        {
            CHAIN_FINALIZED_EXECUTION_BLOCK_HEIGHT_ON_ETH.inc_by(cmp::max(
                0,
                last_block_number as i64 - CHAIN_FINALIZED_EXECUTION_BLOCK_HEIGHT_ON_ETH.get(),
            ));
        }

        Ok(last_finalized_slot_on_eth)
    }

    pub fn run(&mut self, max_iterations: Option<u64>) {
        info!(target: "relay", "=== Relay running ===");
        let mut iter_id = 0;
        while !self.terminate {
            iter_id += 1;
            self.set_terminate(iter_id, max_iterations);
            skip_fail!(
                self.wait_for_synchronization(),
                "Fail to get sync status",
                self.sleep_time_on_sync_secs
            );

            info!(target: "relay", "== New relay loop ==");
            sleep(Duration::from_secs(12));

            let client_mode: ClientMode = skip_fail!(self.eth_client_contract.get_client_mode(),
                "Fail to get client mode",
                self.sleep_time_on_sync_secs
            );

            let were_submission_on_iter = match client_mode {
                ClientMode::SubmitLightClientUpdate => self.submit_light_client_update(),
                ClientMode::SubmitHeader => self.submit_headers()
            };

            if !were_submission_on_iter {
                info!(target: "relay", "Sync with ETH network. Sleep {} secs", self.sleep_time_on_sync_secs);
                thread::sleep(Duration::from_secs(self.sleep_time_on_sync_secs));
            }
        }
    }

    fn submit_light_client_update(&mut self) -> bool {
        info!(target: "relay", "Submit Light Client Update mode");
        self.send_light_client_updates_with_checks()
    }

    fn get_slot_by_block_number(&self, block_number: u64, max_slot: u64, min_slot: u64) -> Result<u64, Box<dyn Error>> {
        let mut left_bound = max_slot;
        let mut right_bound = max_slot;
        let mut step = 1;
        while left_bound >= min_slot {
            left_bound = self.beacon_rpc_client.get_left_non_empty_slot(left_bound)?;
            let left_bound_block_number = self.beacon_rpc_client.get_block_number_for_slot(Slot::new(left_bound))?;
            if left_bound_block_number == block_number {
                return Ok(left_bound);
            } else if left_bound_block_number > block_number {
                right_bound = left_bound;
                left_bound = max(left_bound - step, min_slot - 1);
                step *= 2;
            } else {
                break;
            }
        }

        while left_bound + 1 < right_bound {
            let mid_slot = left_bound + (right_bound - left_bound)/2;
            let mid_non_empty_slot = self.beacon_rpc_client.get_left_non_empty_slot(mid_slot)?;

            if mid_non_empty_slot <= left_bound {
                return Err(Box::new(SlotByBlockNumberNotFound));
            }

            let mid_block_number = self.beacon_rpc_client.get_block_number_for_slot(Slot::new(mid_non_empty_slot))?;
            if mid_block_number == block_number {
                return Ok(mid_non_empty_slot);
            } else if mid_block_number > block_number {
                right_bound = mid_non_empty_slot;
            } else {
                left_bound = mid_slot;
            }
        }

        return Err(Box::new(SlotByBlockNumberNotFound));
    }

    fn submit_headers(&mut self) -> bool {
        info!(target: "relay", "Submit Headers mode");
        let max_slot = return_val_on_fail!(self.eth_client_contract.get_finalized_beacon_block_slot(),
            "Fail to get last finalized slot",
            false
        );

        let min_block_number = return_val_on_fail!(self.eth_client_contract.get_last_block_number(),
                                               "Fail to get last block number",
                                               false);
        let min_slot = return_val_on_fail!(self.get_slot_by_block_number(min_block_number, max_slot, 1),
            "Fail to get slot by block number",
            false);
        let mut current_slot = max_slot;

        info!(target: "relay", "Submit headers from {} to {}", max_slot, min_slot);

        while current_slot > min_slot {
            info!(target: "relay", "= Creating headers batch =");

            let max_block_number = skip_fail!(self.eth_client_contract.get_unfinalized_tail_block_number(),
                "Fail to fetch unfinalized tail block number",
                self.sleep_time_on_sync_secs);

            if let Some(max_block_number) = max_block_number {
                current_slot = skip_fail!(self.get_slot_by_block_number(max_block_number, current_slot + 1, min_slot), "Fail to get slot by block_number", 0) - 1;
            }

            let (headers, new_current_slot) = skip_fail!(
                    self.get_execution_blocks_between(
                        current_slot,
                        min_slot,
                    ),
                    "Network problems during fetching execution blocks",
                    self.sleep_time_on_sync_secs
                );

            if !self.submit_execution_blocks(headers, new_current_slot + 1, &mut current_slot) {
                return false;
            }
        }

        return true;
    }

    fn wait_for_synchronization(&self) -> Result<(), Box<dyn Error>> {
        while self.beacon_rpc_client.is_syncing()?
            || self.eth1_rpc_client.is_syncing()?
            || self.near_rpc_client.is_syncing()?
        {
            info!(target: "relay", "Waiting for sync...");
            thread::sleep(Duration::from_secs(self.sleep_time_on_sync_secs));
        }
        Ok(())
    }

    fn get_light_client_update_from_file(
        config: &Config,
        beacon_rpc_client: &BeaconRPCClient,
    ) -> Result<Option<LightClientUpdate>, Box<dyn Error>> {
        let mut next_light_client_update: Option<LightClientUpdate> = None;
        if let Some(path_to_attested_state) = config.clone().path_to_attested_state {
            if config.clone().include_next_sync_committee_to_light_client {
                next_light_client_update = Some(
                    HandMadeFinalityLightClientUpdate::get_light_client_update_from_file_with_next_sync_committee(
                        beacon_rpc_client,
                        &path_to_attested_state,
                    ).expect("Error on getting light client update from file"),
                );
            } else {
                next_light_client_update = Some(
                    HandMadeFinalityLightClientUpdate::get_finality_light_client_update_from_file(
                        beacon_rpc_client,
                        &path_to_attested_state,
                    )
                    .expect("Error on getting light client update from file"),
                );
            }
        }
        Ok(next_light_client_update)
    }

    fn set_terminate(&mut self, iter_id: u64, max_iterations: Option<u64>) {
        if let Some(max_iter) = max_iterations {
            if iter_id > max_iter {
                self.terminate = true;
            }
        }
    }

    fn get_execution_blocks_between(
        &self,
        last_slot: u64,
        first_slot: u64,
    ) -> Result<(Vec<BlockHeader>, u64), Box<dyn Error>> {
        let mut headers: Vec<BlockHeader> = vec![];
        let mut current_slot = last_slot;

        let max_submitted_headers = self.headers_batch_size;

        while headers.len() < max_submitted_headers as usize
            && current_slot > first_slot
        {
            debug!(target: "relay", "Try add block header for slot={}, headers len={}/{}", current_slot, headers.len(), self.headers_batch_size);
            match self.get_execution_block_by_slot(current_slot) {
                Ok(eth1_header) => headers.push(eth1_header),
                Err(err) => match err.downcast_ref::<NoBlockForSlotError>() {
                    Some(_) => {
                        current_slot -= 1;
                        continue;
                    }
                    None => return Err(err),
                },
            }
            current_slot -= 1;
        }

        Ok((headers, current_slot))
    }

    fn submit_execution_blocks(
        &mut self,
        headers: Vec<BlockHeader>,
        current_slot: u64,
        last_eth2_slot_on_near: &mut u64,
    ) -> bool {
        info!(target: "relay", "Try submit headers from slot={} to {} to NEAR", *last_eth2_slot_on_near + 1, current_slot - 1);
        let execution_outcome = return_val_on_fail!(
            self.eth_client_contract
                .send_headers(&headers),
            "Error on header submission",
            false
        );

        if let FinalExecutionStatus::Failure(error_message) = execution_outcome.status {
            FAILS_ON_HEADERS_SUBMISSION.inc();
            warn!(target: "relay", "FAIL status on Headers submission. Error: {:?}. Transaction URL: https://explorer.{}.near.org/transactions/{}",
                error_message, self.near_network_name, execution_outcome.transaction.hash);

            return false;
        } else {
            *last_eth2_slot_on_near = current_slot - 1;
            info!(target: "relay", "Successful headers submission! Transaction URL: https://explorer.{}.near.org/transactions/{}",
                                  self.near_network_name, execution_outcome.transaction.hash);

            thread::sleep(Duration::from_secs(self.sleep_time_after_submission_secs));
            return true;
        }
    }

    fn verify_bls_signature_for_finality_update(
        &mut self,
        light_client_update: &LightClientUpdate,
    ) -> Result<bool, Box<dyn Error>> {
        let signature_slot_period =
            BeaconRPCClient::get_period_for_slot(light_client_update.signature_slot);
        let finalized_slot_period = BeaconRPCClient::get_period_for_slot(
            self.eth_client_contract.get_finalized_beacon_block_slot()?,
        );

        let light_client_state = self.eth_client_contract.get_light_client_state()?;

        let sync_committee = if signature_slot_period == finalized_slot_period {
            light_client_state.current_sync_committee
        } else {
            light_client_state.next_sync_committee
        };

        finality_update_verify::is_correct_finality_update(
            &self.ethereum_network,
            light_client_update,
            sync_committee,
        )
    }

    fn get_execution_block_by_slot(&self, slot: u64) -> Result<BlockHeader, Box<dyn Error>> {
        match self
            .beacon_rpc_client
            .get_block_number_for_slot(types::Slot::new(slot))
        {
            Ok(block_number) => self
                .eth1_rpc_client
                .get_block_header_by_number(block_number),
            Err(err) => Err(err),
        }
    }
}

// Implementation of functions for submitting light client updates
impl Eth2NearRelay {
    fn is_enough_blocks_for_light_client_update(
        &self,
        last_finalized_slot_on_near: u64,
        last_finalized_slot_on_eth: u64,
    ) -> bool {
        if (last_finalized_slot_on_eth as i64) - (last_finalized_slot_on_near as i64)
            < (ONE_EPOCH_IN_SLOTS * self.interval_between_light_client_updates_submission_in_epochs)
                as i64
        {
            info!(target: "relay", "Light client update were send less then {} epochs ago. Skipping sending light client update", self.interval_between_light_client_updates_submission_in_epochs);
            return false;
        }

        if last_finalized_slot_on_eth <= last_finalized_slot_on_near {
            info!(target: "relay", "Last finalized slot on Eth equal to last finalized slot on NEAR. Skipping sending light client update.");
            return false;
        }

        true
    }

    fn is_shot_run_mode(&self) -> bool {
        self.next_light_client_update.is_some()
    }

    fn send_light_client_updates_with_checks(&mut self) -> bool {
        let last_finalized_slot_on_near: u64 = return_val_on_fail!(
            self.get_last_finalized_slot_on_near(),
            "Error on getting finalized block slot on NEAR. Skipping sending light client update",
            false
        );

        let last_finalized_slot_on_eth: u64 = return_val_on_fail!(self.get_last_finalized_slot_on_eth(),
                "Error on getting last finalized slot on Ethereum. Skipping sending light client update",
                false).as_u64();

        info!(target: "relay", "last_finalized_slot on near/eth {}/{}", last_finalized_slot_on_near, last_finalized_slot_on_eth);

        if self.is_enough_blocks_for_light_client_update(
            last_finalized_slot_on_near,
            last_finalized_slot_on_eth,
        ) {
            self.send_light_client_updates(
                last_finalized_slot_on_near,
                last_finalized_slot_on_eth,
            );
            return true;
        }

        return false;
    }

    fn send_light_client_updates(
        &mut self,
        last_finalized_slot_on_near: u64,
        last_finalized_slot_on_eth: u64,
    ) {
        info!(target: "relay", "= Sending light client update =");

        if self.is_shot_run_mode() {
            info!(target: "relay", "Try sending light client update from file");
            self.send_light_client_update_from_file();
            return;
        }

        if self.get_light_client_update_by_epoch {
            if self.send_regular_light_client_update_by_epoch(
                last_finalized_slot_on_eth,
                last_finalized_slot_on_near,
            ) {
                return;
            }
        }

        if last_finalized_slot_on_eth
            >= last_finalized_slot_on_near + self.max_blocks_for_finalization
        {
            info!(target: "relay", "Too big gap between slot of finalized block on NEAR and ETH. Sending hand made light client update");
            self.send_hand_made_light_client_update(last_finalized_slot_on_near);
        } else {
            self.send_regular_light_client_update(
                last_finalized_slot_on_eth,
                last_finalized_slot_on_near,
            );
        }
    }

    fn send_light_client_update_from_file(&mut self) {
        if let Some(light_client_update) = self.next_light_client_update.clone() {
            self.send_specific_light_client_update(light_client_update);
            self.terminate = true;
        }
    }

    fn send_regular_light_client_update(
        &mut self,
        last_finalized_slot_on_eth: u64,
        last_finalized_slot_on_near: u64,
    ) {
        let last_eth2_period_on_near_chain =
            BeaconRPCClient::get_period_for_slot(last_finalized_slot_on_near);
        info!(target: "relay", "Last finalized slot/period on near={}/{}", last_finalized_slot_on_near, last_eth2_period_on_near_chain);

        let end_period = BeaconRPCClient::get_period_for_slot(last_finalized_slot_on_eth);
        info!(target: "relay", "Last finalized slot/period on ethereum={}/{}", last_finalized_slot_on_eth, end_period);

        let light_client_update = if end_period == last_eth2_period_on_near_chain {
            debug!(target: "relay", "Finalized period on ETH and NEAR are equal. Don't fetch sync commity update");
            return_on_fail!(
                self.beacon_rpc_client.get_finality_light_client_update(),
                "Error on getting light client update. Skipping sending light client update"
            )
        } else {
            debug!(target: "relay", "Finalized period on ETH and NEAR are different. Fetching sync commity update");
            return_on_fail!(
                self.beacon_rpc_client
                    .get_light_client_update_for_last_period(),
                "Error on getting light client update. Skipping sending light client update"
            )
        };

        self.send_specific_light_client_update(light_client_update);
    }

    fn send_regular_light_client_update_by_epoch(
        &mut self,
        last_finalized_slot_on_eth: u64,
        last_finalized_slot_on_near: u64,
    ) -> bool {
        let last_eth2_period_on_near_chain =
            BeaconRPCClient::get_period_for_slot(last_finalized_slot_on_near);
        info!(target: "relay", "Last finalized slot/period on near={}/{}", last_finalized_slot_on_near, last_eth2_period_on_near_chain);

        let end_period = BeaconRPCClient::get_period_for_slot(last_finalized_slot_on_eth);
        info!(target: "relay", "Last finalized slot/period on ethereum={}/{}", last_finalized_slot_on_eth, end_period);

        let last_epoch = last_finalized_slot_on_near / SLOTS_PER_EPOCH;
        let last_period = last_epoch / EPOCHS_PER_SYNC_COMMITTEE_PERIOD;
        let mut update_epoch =
            last_epoch + self.interval_between_light_client_updates_submission_in_epochs + 2;

        let light_client_update = loop {
            let res = self
                .beacon_rpc_client
                .get_light_client_update_by_epoch(update_epoch);

            if let Ok(res) = res {
                let update_epoch =
                    res.finality_update.header_update.beacon_header.slot / SLOTS_PER_EPOCH;
                let update_period = update_epoch / EPOCHS_PER_SYNC_COMMITTEE_PERIOD;

                if update_period > last_period + 1 {
                    debug!(target: "relay", "Finalized period on ETH and NEAR are different. Fetching sync commity update");
                    let res = return_val_on_fail!(
                        self.beacon_rpc_client
                            .get_light_client_update(update_period),
                        "Error on getting light client update. Skipping sending light client update", false
                    );

                    break res;
                }

                break res;
            }

            warn!(target: "relay", "Error: {}", res.unwrap_err());
            thread::sleep(Duration::from_secs(5));

            update_epoch -= 1;
        };

        self.send_specific_light_client_update(light_client_update)
    }

    fn get_attested_slot(
        &mut self,
        last_finalized_slot_on_near: u64,
    ) -> Result<u64, Box<dyn Error>> {
        const EXPECTED_EPOCHS_BETWEEN_HEAD_AND_FINALIZED_BLOCKS: u64 = 2;
        let next_finalized_slot = last_finalized_slot_on_near
            + self.interval_between_light_client_updates_submission_in_epochs * ONE_EPOCH_IN_SLOTS;
        let attested_slot = next_finalized_slot
            + EXPECTED_EPOCHS_BETWEEN_HEAD_AND_FINALIZED_BLOCKS * ONE_EPOCH_IN_SLOTS;

        let attested_slot: u64 = self
            .beacon_rpc_client
            .get_non_empty_beacon_block_header(attested_slot)?
            .slot
            .into();
        trace!(target: "relay", "Chosen attested slot {}", attested_slot);

        Ok(attested_slot)
    }

    fn send_hand_made_light_client_update(&mut self, last_finalized_slot_on_near: u64) {
        let mut attested_slot = return_on_fail!(
            self.get_attested_slot(last_finalized_slot_on_near),
            "Error on getting attested slot"
        );

        let include_next_sync_committee =
            BeaconRPCClient::get_period_for_slot(last_finalized_slot_on_near)
                != BeaconRPCClient::get_period_for_slot(attested_slot);

        loop {
            let light_client_update = return_on_fail!(
                HandMadeFinalityLightClientUpdate::get_finality_light_client_update(
                    &self.beacon_rpc_client,
                    attested_slot,
                    include_next_sync_committee,
                ),
                format!(
                    "Error on getting hand made light client update for attested slot={}.",
                    attested_slot
                )
            );

            let finality_update_slot = light_client_update
                .finality_update
                .header_update
                .beacon_header
                .slot;

            if finality_update_slot <= last_finalized_slot_on_near {
                info!(target: "relay", "Finality update slot for hand made light client update <= last finality update on NEAR. Increment gap for attested slot and skipping light client update.");
                attested_slot = return_on_fail!(
                    self.get_attested_slot(last_finalized_slot_on_near + ONE_EPOCH_IN_SLOTS),
                    "Error on getting attested slot"
                );
                continue;
            }

            trace!(target: "relay", "Hand made light client update: {:?}", light_client_update);
            self.send_specific_light_client_update(light_client_update);
            return;
        }
    }

    fn send_specific_light_client_update(
        &mut self,
        light_client_update: LightClientUpdate,
    ) -> bool {
        let verification_result = return_val_on_fail!(
                self.verify_bls_signature_for_finality_update(&light_client_update),
                "Error on bls verification. Skip sending the light client update",
                false
            );

        if verification_result {
            info!(target: "relay", "PASS bls signature verification!");
        } else {
            warn!(target: "relay", "NOT PASS bls signature verification. Skip sending this light client update");
            return false;
        }

        let execution_outcome = return_val_on_fail_and_sleep!(
                self.eth_client_contract
                    .send_light_client_update(light_client_update.clone()),
                "Fail to send light client update",
                self.sleep_time_on_sync_secs,
                false
            );

        info!(target: "relay", "Sending light client update");

        if let FinalExecutionStatus::Failure(error_message) = execution_outcome.status {
            FAILS_ON_UPDATES_SUBMISSION.inc();
            warn!(target: "relay", "FAIL status on Light Client Update submission. Error: {:?}", error_message);
        }

        info!(target: "relay", "Successful light client update submission! Transaction URL: https://explorer.{}.near.org/transactions/{}",
                                  self.near_network_name, execution_outcome.transaction.hash);

        let finalized_block_number = return_val_on_fail!(
                self.beacon_rpc_client
                    .get_block_number_for_slot(types::Slot::new(
                        light_client_update
                            .finality_update
                            .header_update
                            .beacon_header
                            .slot
                            .as_u64()
                    )),
                "Fail on getting finalized block number",
                false
            );

        info!(target: "relay", "Finalized block number from light client update = {}", finalized_block_number);
        sleep(Duration::from_secs(self.sleep_time_after_submission_secs));
        return true;
    }
}

#[cfg(test)]
mod tests {
    use crate::config_for_tests::ConfigForTests;
    use crate::eth2near_relay::{Eth2NearRelay, ONE_EPOCH_IN_SLOTS};
    use crate::test_utils::{get_relay, get_relay_from_slot, get_relay_with_update_from_file};
    use eth_rpc_client::beacon_rpc_client::BeaconRPCClient;
    use eth_rpc_client::errors::NoBlockForSlotError;
    use eth_rpc_client::hand_made_finality_light_client_update::HandMadeFinalityLightClientUpdate;
    use eth_types::eth2::LightClientUpdate;
    use eth_types::BlockHeader;
    use std::thread::sleep;
    use std::time::Duration;
    use tree_hash::TreeHash;

    const TIMEOUT_SECONDS: u64 = 30;
    const TIMEOUT_STATE_SECONDS: u64 = 1000;

    fn get_test_config() -> ConfigForTests {
        ConfigForTests::load_from_toml("config_for_tests.toml".try_into().unwrap())
    }

    fn send_execution_blocks_between(relay: &mut Eth2NearRelay, start_slot: u64, end_slot: u64) {
        let mut slot = start_slot;
        let mut blocks: Vec<BlockHeader> = vec![];

        while slot <= end_slot {
            match relay.get_execution_block_by_slot(slot) {
                Ok(block) => {
                    blocks.push(block);
                    slot += 1;
                }
                Err(err) => match err.downcast_ref::<NoBlockForSlotError>() {
                    Some(_) => slot += 1,
                    None => sleep(Duration::from_secs(10)),
                },
            }
        }

        relay
            .eth_client_contract
            .send_headers(&blocks)
            .unwrap();
    }

    fn send_blocks_till_finalized_eth_slot(relay: &mut Eth2NearRelay, finality_slot: u64) -> u64 {
        let mut slot = finality_slot + 1;

        let mut finality_slot_on_eth = relay
            .beacon_rpc_client
            .get_last_finalized_slot_number()
            .unwrap()
            .as_u64();

        let mut blocks: Vec<BlockHeader> = vec![];
        while finality_slot == finality_slot_on_eth || slot <= finality_slot_on_eth {
            if let Ok(block) = relay.get_execution_block_by_slot(slot) {
                blocks.push(block)
            }
            slot += 1;

            finality_slot_on_eth = loop {
                if let Ok(last_slot) = relay.beacon_rpc_client.get_last_finalized_slot_number() {
                    break last_slot.as_u64();
                }
            }
        }

        relay
            .eth_client_contract
            .send_headers(&blocks)
            .unwrap();

        finality_slot_on_eth
    }

    fn get_finalized_slot(relay: &Eth2NearRelay) -> u64 {
        relay
            .eth_client_contract
            .get_finalized_beacon_block_slot()
            .unwrap()
    }

    #[test]
    fn test_submit_zero_headers() {
        let config_for_test = get_test_config();

        let mut relay = get_relay(true, true, &config_for_test);

        let mut end_slot = get_finalized_slot(&relay);
        end_slot += 1;

        let blocks: Vec<BlockHeader> = vec![];
        if let Ok(_) = relay.eth_client_contract.send_headers(&blocks) {
            panic!("No error on submit 0 headers");
        }
    }

    #[test]
    fn test_send_specific_light_client_update() {
        let config_for_test = get_test_config();

        let mut relay = get_relay(true, true, &config_for_test);
        let finalized_slot = get_finalized_slot(&relay);

        let light_client_updates: Vec<LightClientUpdate> = serde_json::from_str(
            &std::fs::read_to_string(config_for_test.path_to_light_client_updates)
                .expect("Unable to read file"),
        )
        .unwrap();

        let finalized_slot_1 = light_client_updates[1]
            .finality_update
            .header_update
            .beacon_header
            .slot;

        send_execution_blocks_between(&mut relay, finalized_slot + 1, finalized_slot_1);

        let finalized_slot = get_finalized_slot(&relay);
        assert_eq!(finalized_slot, config_for_test.first_slot);

        relay.send_specific_light_client_update(light_client_updates[1].clone());

        let finalized_slot = get_finalized_slot(&relay);
        assert_eq!(finalized_slot, finalized_slot_1);
    }

    #[test]
    fn test_finality_light_client_update_correctness() {
        const TREE_FINALITY_DEPTH: usize = 6;
        const TREE_FINALITY_INDEX: usize = 41;
        const TREE_NEXT_SYNC_COMMITTEE_DEPTH: usize = 5;
        const TREE_NEXT_SYNC_COMMITTEE_INDEX: usize = 23;

        let config_for_test = get_test_config();

        let relay = get_relay(true, true, &config_for_test);

        let light_client_update = relay
            .beacon_rpc_client
            .get_light_client_update_for_last_period()
            .unwrap();

        let branch: Vec<ethereum_types::H256> = light_client_update
            .finality_update
            .finality_branch
            .iter()
            .map(|h| h.0)
            .collect();
        assert!(
            merkle_proof::verify_merkle_proof(
                light_client_update
                    .finality_update
                    .header_update
                    .beacon_header
                    .tree_hash_root(),
                branch.as_slice(),
                TREE_FINALITY_DEPTH,
                TREE_FINALITY_INDEX,
                light_client_update.attested_beacon_header.state_root.0
            ),
            "Incorrect proof of inclusion the finality checkpoint to attested beacon state"
        );

        let branch = light_client_update
            .sync_committee_update
            .as_ref()
            .unwrap()
            .next_sync_committee_branch
            .iter()
            .map(|h| h.0)
            .collect::<Vec<ethereum_types::H256>>();
        assert!(
            merkle_proof::verify_merkle_proof(
                light_client_update
                    .sync_committee_update
                    .as_ref()
                    .unwrap()
                    .next_sync_committee
                    .tree_hash_root(),
                branch.as_slice(),
                TREE_NEXT_SYNC_COMMITTEE_DEPTH,
                TREE_NEXT_SYNC_COMMITTEE_INDEX,
                light_client_update.attested_beacon_header.state_root.0
            ),
            "Incorrect proof of inclusion the next sync committee to finality beacon state"
        );
    }

    #[test]
    #[ignore]
    fn test_hand_made_light_client_update() {
        let config_for_test = get_test_config();

        let mut relay = get_relay(true, true, &config_for_test);
        let finalized_slot = get_finalized_slot(&relay);

        let light_client_updates: Vec<LightClientUpdate> = serde_json::from_str(
            &std::fs::read_to_string(config_for_test.path_to_light_client_updates)
                .expect("Unable to read file"),
        )
        .unwrap();
        let finalized_slot_1 = light_client_updates[1]
            .finality_update
            .header_update
            .beacon_header
            .slot;

        send_execution_blocks_between(&mut relay, finalized_slot + 1, finalized_slot_1);

        let finalized_slot = get_finalized_slot(&relay);
        assert_eq!(finalized_slot, config_for_test.first_slot);

        relay.send_hand_made_light_client_update(finalized_slot);

        let finalized_slot = get_finalized_slot(&relay);
        assert_eq!(finalized_slot, finalized_slot_1);
    }

    #[test]
    #[ignore]
    fn test_hand_made_light_client_update_with_null_signature_slot() {
        let config_for_test = get_test_config();

        let mut relay = get_relay(true, true, &config_for_test);
        let finalized_slot = get_finalized_slot(&relay);

        let light_client_updates: Vec<LightClientUpdate> = serde_json::from_str(
            &std::fs::read_to_string(config_for_test.path_to_light_client_updates)
                .expect("Unable to read file"),
        )
        .unwrap();
        let finalized_slot_1 = light_client_updates[1]
            .finality_update
            .header_update
            .beacon_header
            .slot;

        send_execution_blocks_between(&mut relay, finalized_slot + 1, finalized_slot_1);

        let finalized_slot = get_finalized_slot(&relay);
        assert_eq!(finalized_slot, config_for_test.first_slot);
        let attested_slot = relay
            .get_attested_slot(config_for_test.slot_without_block_2 - ONE_EPOCH_IN_SLOTS * 3 - 1)
            .unwrap();
        if relay.get_execution_block_by_slot(attested_slot + 1).is_ok() {
            panic!("Signature slot has block {}", attested_slot + 1);
        }

        relay.send_hand_made_light_client_update(
            config_for_test.slot_without_block_2 - ONE_EPOCH_IN_SLOTS * 3 - 1,
        );

        let finalized_slot = get_finalized_slot(&relay);
        assert_eq!(finalized_slot, finalized_slot_1);
    }

    #[test]
    #[ignore]
    fn test_send_light_client_update() {
        let config_for_test = get_test_config();

        let mut relay = get_relay(true, false, &config_for_test);
        let finality_slot = get_finalized_slot(&relay);

        let _finality_slot_on_eth = send_blocks_till_finalized_eth_slot(&mut relay, finality_slot);
        assert!(relay.send_light_client_updates_with_checks());

        let new_finalized_slot = get_finalized_slot(&relay);
        assert_ne!(finality_slot, new_finalized_slot);
    }

    #[test]
    fn test_get_execution_block_by_slot() {
        let config_for_test = get_test_config();

        let mut relay = get_relay(true, true, &config_for_test);
        relay
            .get_execution_block_by_slot(config_for_test.slot_without_block - 1)
            .unwrap();
        if let Err(err) = relay.get_execution_block_by_slot(config_for_test.slot_without_block) {
            if err.downcast_ref::<NoBlockForSlotError>().is_none() {
                panic!("Wrong error type for slot without block");
            }
        } else {
            panic!("Return execution block for slot without block");
        }

        relay.beacon_rpc_client = BeaconRPCClient::new(
            "http://httpstat.us/504/",
            TIMEOUT_SECONDS,
            TIMEOUT_STATE_SECONDS,
            None,
        );
        if let Err(err) = relay.get_execution_block_by_slot(config_for_test.slot_without_block) {
            if err.downcast_ref::<NoBlockForSlotError>().is_some() {
                panic!("Wrong error type for unworking network");
            }
        } else {
            panic!("Return execution block in unworking network");
        }
    }

    #[test]
    fn test_verify_bls_signature() {
        let config_for_test = get_test_config();
        let mut relay = get_relay(true, true, &config_for_test);
        let mut light_client_updates: Vec<LightClientUpdate> = serde_json::from_str(
            &std::fs::read_to_string(config_for_test.path_to_light_client_updates)
                .expect("Unable to read file"),
        )
        .unwrap();

        assert!(relay
            .verify_bls_signature_for_finality_update(&light_client_updates[1])
            .unwrap());

        light_client_updates[1].attested_beacon_header =
            light_client_updates[0].attested_beacon_header.clone();

        assert!(!relay
            .verify_bls_signature_for_finality_update(&light_client_updates[1])
            .unwrap());
    }

    #[test]
    #[ignore]
    fn test_get_attested_slot() {
        let config_for_test = get_test_config();

        let mut relay = get_relay(true, true, &config_for_test);

        let light_client_updates: Vec<LightClientUpdate> = serde_json::from_str(
            &std::fs::read_to_string(config_for_test.path_to_light_client_updates)
                .expect("Unable to read file"),
        )
        .unwrap();
        let finalized_slot = light_client_updates[2]
            .finality_update
            .header_update
            .beacon_header
            .slot;
        let attested_slot = relay.get_attested_slot(finalized_slot).unwrap();

        match HandMadeFinalityLightClientUpdate::get_finality_light_client_update(
            &relay.beacon_rpc_client,
            attested_slot,
            false,
        ) {
            Ok(light_client_update) => {
                let finality_update_slot = light_client_update
                    .finality_update
                    .header_update
                    .beacon_header
                    .slot;

                assert!(finality_update_slot > finalized_slot);
            }
            Err(_) => {
                panic!("Error on get light client update");
            }
        }
    }

    #[test]
    fn test_submit_execution_blocks() {
        let config_for_test = get_test_config();
        let mut relay = get_relay(true, true, &config_for_test);
        let mut finalized_slot = get_finalized_slot(&relay);
        let blocks = relay
            .get_execution_blocks_between(
                finalized_slot + 1,
                config_for_test.right_bound_in_slot_search,
            )
            .unwrap();
        relay.submit_execution_blocks(blocks.0, blocks.1, &mut finalized_slot);
        assert_eq!(finalized_slot, blocks.1 - 1);
    }

    #[test]
    #[ignore]
    fn try_submit_update_with_not_enough_blocks() {
        let config_for_test = get_test_config();
        let mut relay = get_relay(true, true, &config_for_test);
        let finalized_slot = get_finalized_slot(&relay);

        let light_client_updates: Vec<LightClientUpdate> = serde_json::from_str(
            &std::fs::read_to_string(config_for_test.path_to_light_client_updates)
                .expect("Unable to read file"),
        )
        .unwrap();
        let finalized_slot_1 = light_client_updates[1]
            .finality_update
            .header_update
            .beacon_header
            .slot;

        send_execution_blocks_between(&mut relay, finalized_slot + 1, finalized_slot_1 - 1);

        let finalized_slot = get_finalized_slot(&relay);
        assert_eq!(finalized_slot, config_for_test.first_slot);

        assert!(!relay.send_light_client_updates_with_checks());
        let finalized_slot = get_finalized_slot(&relay);

        assert_eq!(finalized_slot, config_for_test.first_slot);
    }

    #[test]
    fn test_not_invalid_attested_slot() {
        let config_for_test = get_test_config();

        let mut relay = get_relay(true, true, &config_for_test);
        let finalized_slot = config_for_test.first_slot;
        let possible_attested_slot = finalized_slot
            + ONE_EPOCH_IN_SLOTS * 2
            + ONE_EPOCH_IN_SLOTS * relay.interval_between_light_client_updates_submission_in_epochs;
        if relay
            .get_execution_block_by_slot(possible_attested_slot)
            .is_ok()
        {
            panic!("possible attested slot has execution block");
        }

        let attested_slot = relay.get_attested_slot(finalized_slot).unwrap();
        relay.get_execution_block_by_slot(attested_slot).unwrap();
    }

    #[test]
    #[ignore]
    fn test_send_regular_light_client_update() {
        let config_for_test = get_test_config();
        let mut relay = get_relay(true, false, &config_for_test);
        let finality_slot = get_finalized_slot(&relay);
        let finality_slot_on_eth = send_blocks_till_finalized_eth_slot(&mut relay, finality_slot);
        relay.send_regular_light_client_update(finality_slot_on_eth, finality_slot);

        let new_finalized_slot = get_finalized_slot(&relay);
        assert_ne!(finality_slot, new_finalized_slot);
    }

    #[test]
    fn test_too_often_updates() {
        let config_for_test = get_test_config();
        let mut relay = get_relay(true, false, &config_for_test);
        relay.interval_between_light_client_updates_submission_in_epochs = 2;

        let finality_slot = get_finalized_slot(&relay);

        let _finality_slot_on_eth = send_blocks_till_finalized_eth_slot(&mut relay, finality_slot);
        assert!(!relay.send_light_client_updates_with_checks());

        let new_finalized_slot = get_finalized_slot(&relay);
        assert_eq!(finality_slot, new_finalized_slot);
    }

    #[test]
    #[ignore]
    fn test_run() {
        let config_for_test = get_test_config();
        let mut relay = get_relay(true, true, &config_for_test);
        let finality_slot = get_finalized_slot(&relay);

        relay.run(Some(5));

        let new_finality_slot = get_finalized_slot(&relay);

        assert_ne!(finality_slot, new_finality_slot);
    }

    #[test]
    #[ignore]
    fn test_base_update_for_new_period() {
        let config_for_test = get_test_config();
        let mut relay = get_relay_from_slot(
            true,
            config_for_test.finalized_slot_before_new_period,
            &config_for_test,
        );
        relay.headers_batch_size = 33;
        relay.max_blocks_for_finalization = 100;

        let blocks = relay
            .get_execution_blocks_between(
                config_for_test.finalized_slot_before_new_period + 1,
                config_for_test.finalized_slot_before_new_period + 100,
            )
            .unwrap();
        let mut last_slot_on_near = config_for_test.finalized_slot_before_new_period;
        let finality_slot = get_finalized_slot(&relay);

        assert_eq!(finality_slot, last_slot_on_near);

        relay.submit_execution_blocks(blocks.0, blocks.1, &mut last_slot_on_near);

        assert!(relay.send_light_client_updates_with_checks());

        let new_finality_slot = get_finalized_slot(&relay);

        assert_ne!(
            config_for_test.finalized_slot_before_new_period,
            new_finality_slot
        );
        assert_eq!(
            BeaconRPCClient::get_period_for_slot(new_finality_slot),
            BeaconRPCClient::get_period_for_slot(config_for_test.finalized_slot_before_new_period)
                + 1
        );
    }

    #[test]
    #[ignore]
    fn test_base_update_for_same_period() {
        let config_for_test = get_test_config();
        let init_slot = config_for_test.finalized_slot_before_new_period - ONE_EPOCH_IN_SLOTS - 1;
        let mut relay = get_relay_from_slot(true, init_slot, &config_for_test);
        relay.headers_batch_size = 33;
        relay.max_blocks_for_finalization = 100;

        let blocks = relay
            .get_execution_blocks_between(
                init_slot + 1,
                config_for_test.finalized_slot_before_new_period,
            )
            .unwrap();
        let mut last_slot_on_near = init_slot;
        let finality_slot = get_finalized_slot(&relay);

        assert_eq!(finality_slot, last_slot_on_near);

        relay.submit_execution_blocks(blocks.0, blocks.1, &mut last_slot_on_near);

        assert!(relay.send_light_client_updates_with_checks());

        let new_finality_slot = get_finalized_slot(&relay);

        assert_ne!(init_slot, new_finality_slot);
    }

    #[test]
    #[ignore]
    fn test_update_new_period_without_next_sync_committee() {
        let config_for_test = get_test_config();
        let mut relay = get_relay_from_slot(
            true,
            config_for_test.finalized_slot_before_new_period,
            &config_for_test,
        );
        relay.headers_batch_size = 33;
        let blocks = relay
            .get_execution_blocks_between(
                config_for_test.finalized_slot_before_new_period + 1,
                config_for_test.finalized_slot_before_new_period + 100,
            )
            .unwrap();
        let mut last_slot_on_near = config_for_test.finalized_slot_before_new_period;

        relay.submit_execution_blocks(blocks.0, blocks.1, &mut last_slot_on_near);

        let attested_slot = relay
            .get_attested_slot(config_for_test.finalized_slot_before_new_period)
            .unwrap();
        let light_client_update =
            HandMadeFinalityLightClientUpdate::get_finality_light_client_update(
                &relay.beacon_rpc_client,
                attested_slot,
                false,
            )
            .unwrap();

        relay.send_specific_light_client_update(light_client_update);

        let new_finality_slot = get_finalized_slot(&relay);

        assert_eq!(
            config_for_test.finalized_slot_before_new_period,
            new_finality_slot
        );
    }

    #[test]
    #[ignore]
    fn test_send_light_client_update_from_file() {
        let config_for_test = get_test_config();
        let mut relay = get_relay_with_update_from_file(true, true, false, &config_for_test);
        let finality_slot = get_finalized_slot(&relay);
        relay.run(None);

        let new_finality_slot = get_finalized_slot(&relay);
        assert_ne!(finality_slot, new_finality_slot);
    }

    #[test]
    #[ignore]
    fn test_send_light_client_update_from_file_with_next_sync_committee() {
        let config_for_test = get_test_config();
        let mut relay = get_relay_with_update_from_file(true, true, true, &config_for_test);
        let finality_slot = get_finalized_slot(&relay);
        relay.run(None);

        let new_finality_slot = get_finalized_slot(&relay);
        assert_ne!(finality_slot, new_finality_slot);
    }

    #[test]
    #[ignore]
    fn test_max_finalized_blocks_8_epochs() {
        let config_for_test = get_test_config();
        let mut relay = get_relay(true, true, &config_for_test);
        relay.max_blocks_for_finalization = 10000;
        relay.headers_batch_size = 10000;

        let light_client_updates: Vec<LightClientUpdate> = serde_json::from_str(
            &std::fs::read_to_string(config_for_test.path_to_light_client_updates)
                .expect("Unable to read file"),
        )
        .unwrap();
        let finalized_slot_8 = light_client_updates[8]
            .finality_update
            .header_update
            .beacon_header
            .slot;

        let finalized_slot = get_finalized_slot(&relay);
        send_execution_blocks_between(&mut relay, finalized_slot + 1, finalized_slot_8);

        relay.send_specific_light_client_update(light_client_updates[8].clone());

        let finalized_slot = get_finalized_slot(&relay);
        assert_eq!(finalized_slot, finalized_slot_8);
    }

    #[test]
    #[ignore]
    #[should_panic]
    // Can't finalize 393 blocks
    fn test_max_finalized_blocks_9_epochs() {
        let config_for_test = get_test_config();
        let mut relay = get_relay(true, true, &get_test_config());
        relay.max_blocks_for_finalization = 10000;
        relay.headers_batch_size = 10000;

        let light_client_updates: Vec<LightClientUpdate> = serde_json::from_str(
            &std::fs::read_to_string(config_for_test.path_to_light_client_updates)
                .expect("Unable to read file"),
        )
        .unwrap();

        let finalized_slot_9 = light_client_updates[9]
            .finality_update
            .header_update
            .beacon_header
            .slot;

        let finalized_slot = get_finalized_slot(&relay);
        send_execution_blocks_between(&mut relay, finalized_slot + 1, finalized_slot_9);

        relay.send_specific_light_client_update(light_client_updates[9].clone());

        let finalized_slot = get_finalized_slot(&relay);
        assert_eq!(finalized_slot, finalized_slot_9);
    }
}
