use crate::{
    U256,
    error::{BtcError, Result},
    sha256::Hash,
    types::*,
    util::*,
};

use std::collections::{HashMap, HashSet};
use std::io::{Error as IoError, ErrorKind as IoErrorKind, Read, Result as IoResult, Write};

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Blockchain {
    utxos: HashMap<Hash, (TransactionOutput, bool)>,
    blocks: Vec<Block>,
    target: U256,
    /// The mempool is a list of transactions that have been sent to the network and haven’t
    /// been processed yet.
    #[serde(default, skip_serializing)]
    mempool: Vec<(Transaction, DateTime<Utc>)>,
}

impl Default for Blockchain {
    fn default() -> Self {
        Self::new()
    }
}

impl Blockchain {
    pub fn new() -> Self {
        Blockchain {
            utxos: HashMap::new(),
            blocks: vec![],
            target: crate::MIN_TARGET,
            mempool: vec![],
        }
    }

    // TODO: in two conficting transactions (what does that mean?), remove the one with smaller
    // fee.
    pub fn add_to_mempool(&mut self, transaction: Transaction) -> Result<()> {
        // validate before inserting transaction to mempool, all inputs must match known UTXOs, and
        // must be unique
        let mut known_inputs = HashSet::new();
        for input in &transaction.inputs {
            if !self.utxos.contains_key(&input.prev_transaction_output_hash) {
                return Err(BtcError::InvalidTransaction);
            }

            if known_inputs.contains(&input.prev_transaction_output_hash) {
                return Err(BtcError::InvalidTransaction);
            }

            known_inputs.insert(input.prev_transaction_output_hash);
        }

        let mut to_remove: Vec<usize> = Vec::new();

        // check if any of the utxos have the bool mark set to true and if so, find the transaction
        // that references them in mempool, remove it and set all the utxos it references to false
        for input in &transaction.inputs {
            if let Some((_, true)) = self.utxos.get(&input.prev_transaction_output_hash) {
                // find a mempool tx that outputs this UTXO
                if let Some((idx, _referencing_idx)) =
                    self.mempool
                        .iter()
                        .enumerate()
                        .find(|(_idx, (tx, _txtime))| {
                            tx.outputs
                                .iter()
                                .any(|output| output.hash() == input.prev_transaction_output_hash)
                        })
                {
                    to_remove.push(idx);
                } else {
                    // if there is no matching transaction set this utxo to false
                    self.utxos
                        .entry(input.prev_transaction_output_hash)
                        .and_modify(|(_transaction, marked)| *marked = false);
                }
            }
        }

        to_remove.sort_unstable();
        to_remove.dedup();
        for idx in to_remove.into_iter().rev() {
            // remove returns the transaction so we can unmark its inputs
            let (referencing_transaction, _txtime) = self.mempool.remove(idx);
            for input in &referencing_transaction.inputs {
                self.utxos
                    .entry(input.prev_transaction_output_hash)
                    .and_modify(|(_tx, marked)| *marked = false);
            }
        }

        let all_inputs = transaction
            .inputs
            .iter()
            .map(|input| {
                self.utxos
                    .get(&input.prev_transaction_output_hash)
                    .expect("BUG: Impossible")
                    .0
                    .value
            })
            .sum::<u64>();

        let all_outputs: u64 = transaction.outputs.iter().map(|output| output.value).sum();

        // all inputs be lower than all outp[uts
        if all_inputs < all_outputs {
            println!("Inputs are lower than outputs");
            return Err(BtcError::InvalidTransaction);
        }

        for input in &transaction.inputs {
            self.utxos
                .entry(input.prev_transaction_output_hash)
                .and_modify(|(_tx, marked)| {
                    *marked = true;
                });
        }

        self.mempool.push((transaction, Utc::now()));

        // sort by miner fee
        self.mempool.sort_by_key(|(transaction, _)| {
            let all_inputs = transaction
                .inputs
                .iter()
                .map(|input| {
                    self.utxos
                        .get(&input.prev_transaction_output_hash)
                        .expect("BUG: Impossible")
                        .0
                        .value
                })
                .sum::<u64>();

            let all_outputs: u64 = transaction.outputs.iter().map(|output| output.value).sum();

            all_inputs - all_outputs
        });

        Ok(())
    }

    /// remove transactions older than MAX_MEMPOOL_TRANSACTION_AGE
    pub fn cleanup_mempool(&mut self) {
        let now = Utc::now();
        let mut utxo_hashes_to_unmark = vec![];
        self.mempool().to_vec().retain(|(transaction, timestamp)| {
            if now - *timestamp
                > chrono::Duration::seconds(crate::MAX_MEMPOOL_TRANSACTION_AGE as i64)
            {
                utxo_hashes_to_unmark.extend(
                    transaction
                        .inputs
                        .iter()
                        .map(|input| input.prev_transaction_output_hash),
                );
                false
            } else {
                true
            }
        });

        // unmark all of the UTXOs
        for hash in utxo_hashes_to_unmark {
            self.utxos
                .entry(hash)
                .and_modify(|(_tx, marked)| *marked = false);
        }
    }

    pub fn mempool(&self) -> &[(Transaction, DateTime<Utc>)] {
        // later, we will also need to keep track
        // of time
        &self.mempool
    }

    /// utxos
    pub fn utxos(&self) -> &HashMap<Hash, (TransactionOutput, bool)> {
        &self.utxos
    }

    /// target
    pub fn target(&self) -> U256 {
        self.target
    }

    /// blocks
    pub fn blocks(&self) -> impl Iterator<Item = &Block> {
        self.blocks.iter()
    }

    // types.rs
    // block height
    pub fn block_height(&self) -> u64 {
        self.blocks.len() as u64
    }

    /// Rebuild UTXO set from the blockchain
    /// For every block in the blockchain, we go
    /// through every transaction, and for every transaction, we go through every input
    /// and output. We add all outputs we see and remove the outputs if we see an input
    /// that spends it.
    pub fn rebuild_utxos(&mut self) {
        for block in &self.blocks {
            for transaction in &block.transactions {
                for input in &transaction.inputs {
                    self.utxos.remove(&input.prev_transaction_output_hash);
                }

                for output in transaction.outputs.iter() {
                    self.utxos
                        .insert(transaction.hash(), (output.clone(), false));
                }
            }
        }
    }

    pub fn add_block(&mut self, block: Block) -> Result<()> {
        if self.blocks.is_empty() {
            if block.header.prev_block_hash != Hash::zero() {
                println!(
                    "First block but previous block hash isn't zero, therefore block is invalid"
                );
                return Err(BtcError::InvalidBlock);
            }
        } else {
            // make sure the previous hash matches
            let last_block = self.blocks.last().unwrap();
            if block.header.prev_block_hash != last_block.hash() {
                println!("Previous hash is wrong, block is invalid");
            }

            // check if hash is less than target
            if !block.header.hash().matches_target(block.header.target) {
                println!("Block hash is higher than network target, block is invalid!");
                return Err(BtcError::InvalidBlock);
            }

            // check if block's merkel root hash is correct
            let calculated_merkle_root = MerkleRoot::calculate(&block.transactions);

            if calculated_merkle_root != block.header.merkle_root {
                println!("Merkle root does not match, block is invalid!");
                return Err(BtcError::InvalidMerkleRoot);
            }

            // check if the timestamp of the last block is higher than current block
            if block.header.timestamp <= last_block.header.timestamp {
                println!("Timestamp is incorrect, invalid block!");
                return Err(BtcError::InvalidBlock);
            }

            block
                .verify_transactions(self.block_height(), self.utxos())
                .unwrap();
        }

        // Remove transactinos from the mempool that are now in the block
        let block_transactions: HashSet<_> =
            block.transactions.iter().map(|tx| tx.hash()).collect();

        self.mempool
            .retain(|tx| !block_transactions.contains(&tx.0.hash()));
        self.blocks.push(block);
        self.try_adjust_target();
        Ok(())
    }

    /// try to adjust the target of the blockchain
    pub fn try_adjust_target(&mut self) {
        if self.blocks.is_empty() {
            return;
        }

        if !self
            .blocks
            .len()
            .is_multiple_of(crate::DIFFICULTY_UPDATE_INTERVAL as usize)
        {
            return;
        }

        // measure the time it took to mine the last blocks
        let start_time = self.blocks
            [self.blocks.len() - crate::DIFFICULTY_UPDATE_INTERVAL as usize]
            .header
            .timestamp;
        let end_time = self.blocks.last().unwrap().header.timestamp;

        let time_diff = end_time - start_time;
        let time_diff_seconds = time_diff.num_seconds();

        let target_seconds = crate::IDEAL_BLOCK_TIME * crate::DIFFICULTY_UPDATE_INTERVAL;
        // multiply the current target by actual time divided by
        // ideal time
        // NewTarget = OldTarget * (ActualTime / IdealTime)
        let new_target = BigDecimal::parse_bytes(self.target.to_string().as_bytes(), 10)
            .expect("BUG: Impossible")
            * (BigDecimal::from(time_diff_seconds) / BigDecimal::from(target_seconds));

        // cut of decimal point and everything after it from string repesentation of new_target
        let new_target_str = new_target
            .to_string()
            .split('.')
            .next()
            .expect("BUG: Expected a decimal point")
            .to_owned();

        let new_target: U256 = U256::from_str_radix(&new_target_str, 10).expect("BUG: Impossible");

        // clamp new_target to be within the range of the 4 * self.target and self.target / 4
        // it seems like bitcoin does not want to adjust the difficulty by more than a factor of 4x
        // in either direction, thats why the multiplicatoin or divisoin by 4
        let new_target = if new_target < self.target / 4 {
            self.target / 4
        } else if new_target > self.target * 4 {
            self.target * 4
        } else {
            new_target
        };

        // if the new target is more than the minimum target, set it to the minimum target
        self.target = new_target.min(crate::MIN_TARGET);
    }

    pub fn calculate_block_reward(&self) -> u64 {
        let block_height = self.block_height();
        let halvings = block_height / crate::HALVING_INTERVAL;
        (crate::INITIAL_REWARD * 10u64.pow(8)) >> halvings
    }
}

impl Saveable for Blockchain {
    fn load<I: Read>(reader: I) -> IoResult<Self> {
        ciborium::de::from_reader(reader)
            .map_err(|_| IoError::new(IoErrorKind::InvalidData, "Failed to deserialize Block"))
    }
    fn save<O: Write>(&self, writer: O) -> IoResult<()> {
        ciborium::ser::into_writer(self, writer)
            .map_err(|_| IoError::new(IoErrorKind::InvalidData, "Failed to serialize Block"))
    }
}
