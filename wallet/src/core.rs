use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use tracing::*;

use anyhow::{Context, Result};
use btclib::crypto::{PrivateKey, PublicKey, Signature};
use btclib::network::Message;
use btclib::types::{Transaction, TransactionInput, TransactionOutput};
use btclib::util::Saveable;

use crossbeam_skiplist::SkipMap;
use kanal::{AsyncSender, Sender};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Key {
    public: PathBuf,
    private: PathBuf,
}

#[derive(Debug, Clone)]
struct LoadedKey {
    public: PublicKey,
    private: PrivateKey,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Recipient {
    pub name: String,
    pub key: PathBuf,
}

#[derive(Clone)]
pub struct LoadedRecipient {
    pub name: String,
    pub key: PublicKey,
}

impl Recipient {
    pub fn load(&self) -> Result<LoadedRecipient> {
        let key = PublicKey::load_from_file(&self.key)?;
        Ok(LoadedRecipient {
            name: self.name.clone(),
            key,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FeeType {
    Fixed,
    Percent,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FeeConfig {
    pub fee_type: FeeType,
    pub value: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub keys: Vec<Key>,
    pub contacts: Vec<Recipient>,
    pub default_node: String,
    pub fee_config: FeeConfig,
}

#[derive(Debug, Clone)]
pub struct UtxoStore {
    keys: Vec<LoadedKey>,
    utxos: Arc<SkipMap<PublicKey, Vec<(TransactionOutput, bool)>>>,
}

impl UtxoStore {
    fn new() -> Self {
        UtxoStore {
            keys: Vec::new(),
            utxos: Arc::new(SkipMap::new()),
        }
    }

    fn add_key(&mut self, key: LoadedKey) {
        self.keys.push(key);
    }
}

#[derive(Debug)]
pub struct Core {
    pub config: Config,
    utxos: UtxoStore,
    pub tx_sender: Sender<Transaction>,
    pub stream: Mutex<TcpStream>,
}

impl Core {
    fn new(config: Config, utxos: UtxoStore, stream: TcpStream) -> Self {
        let (tx_sender, _) = kanal::bounded(10);
        Core {
            config,
            utxos,
            tx_sender,
            stream: Mutex::new(stream),
        }
    }

    pub async fn load_config(config_path: PathBuf) -> Result<Self> {
        info!("Loading core from config: {:?}", config_path);
        let config: Config = toml::from_str(&fs::read_to_string(&config_path)?)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;
        let mut utxos = UtxoStore::new();
        let stream = TcpStream::connect(&config.default_node).await?;
        // load keys from config
        for key in &config.keys {
            debug!("Loading key pair: {:?}", key.public);
            let public = PublicKey::load_from_file(&key.public)
                .with_context(|| "Failed to load public key specified in the file")?;
            let private = PrivateKey::load_from_file(&key.private)
                .with_context(|| "Failed to load private key specified in the file")?;
            utxos.add_key(LoadedKey { public, private });
        }

        Ok(Core::new(config, utxos, stream))
    }

    /// Fetch UTXOs from the node for all loaded keys
    pub async fn fetch_utxos(&self) -> Result<()> {
        debug!("Fetching UTXOs from node: {}", self.config.default_node);
        for key in &self.utxos.keys {
            let message = Message::FetchUTXOs(key.public.clone());
            message.send_async(&mut *self.stream.lock().await).await?;
            if let Message::UTXOs(utxos) =
                Message::receive_async(&mut *self.stream.lock().await).await?
            {
                debug!("Received {} UTXOs for key: {}", utxos.len(), key.public);
                // replace the entire UTXO set for this key
                self.utxos.utxos.insert(
                    key.public.clone(),
                    utxos
                        .into_iter()
                        .map(|(marked, output)| (output, marked))
                        .collect(),
                );
            } else {
                error!("Unexpected response from node");
                return Err(anyhow::anyhow!("Unexpected response from node"));
            }
        }
        info!("UTXOs fetched successfully!");
        Ok(())
    }

    /// Send a transaction to the node
    pub async fn send_transaction(&self, transaction: Transaction) -> Result<()> {
        debug!("Sending transaction to node: {}", self.config.default_node);
        let message = Message::SubmitTransaction(transaction);
        message.send_async(&mut *self.stream.lock().await).await?;
        info!("Transaction sent successfully!");
        Ok(())
    }

    /// Send a transaction to the node
    pub fn send_transaction_async(&self, recipient: &str, amount: u64) -> Result<()> {
        info!("Preparing to send {} satoshis to {}", amount, recipient);
        let recipient_key = self
            .config
            .contacts
            .iter()
            .find(|r| r.name == recipient)
            .ok_or_else(|| anyhow::anyhow!("Recipient not found"))?
            .load()?
            .key;
        let transaction = self.create_transaction(&recipient_key, amount)?;
        debug!("Sending transaction asynchronously");
        self.tx_sender.send(transaction)?;
        info!("Transaction sent successfully!");
        Ok(())
    }

    pub fn create_transaction(&self, recipient: &PublicKey, amount: u64) -> Result<Transaction> {
        debug!(
            "Creating transaction for {} satoshis to {:?}",
            amount, recipient
        );
        let fee = self.calculate_fee(amount);
        let total_amount = amount + fee;
        let mut inputs = Vec::new();
        let mut input_sum = 0;
        for entry in self.utxos.utxos.iter() {
            let pubkey = entry.key();
            let utxos = entry.value();
            for (utxo, marked) in utxos.iter() {
                if *marked {
                    continue; // Skip marked UTXOs
                }
                if input_sum >= total_amount {
                    break;
                }
                inputs.push(TransactionInput {
                    prev_transaction_output_hash: utxo.hash(),
                    signature: Signature::sign_output(
                        &utxo.hash(),
                        &self
                            .utxos
                            .keys
                            .iter()
                            .find(|k| k.public == *pubkey)
                            .unwrap()
                            .private,
                    ),
                });
                input_sum += utxo.value;
            }
            if input_sum >= total_amount {
                break;
            }
        }

        if input_sum < total_amount {
            return Err(anyhow::anyhow!("Insufficient funds"));
        }

        let mut outputs = vec![TransactionOutput {
            value: amount,
            unique_id: uuid::Uuid::new_v4(),
            pubkey: self.utxos.keys[0].public.clone(),
        }];

        if input_sum > total_amount {
            outputs.push(TransactionOutput {
                value: input_sum - total_amount,
                unique_id: uuid::Uuid::new_v4(),
                pubkey: self.utxos.keys[0].public.clone(),
            });
        }

        Ok(Transaction::new(inputs, outputs))
    }

    /// Calculate fee noooo :(
    pub fn calculate_fee(&self, amount: u64) -> u64 {
        match self.config.fee_config.fee_type {
            FeeType::Fixed => self.config.fee_config.value as u64,
            FeeType::Percent => (amount as f64 * self.config.fee_config.value / 100.0) as u64,
        }
    }

    /// Get the current balance yeeyy
    pub fn get_balance(&self) -> u64 {
        let balance = self
            .utxos
            .utxos
            .iter()
            .map(|entry| entry.value().iter().map(|utxo| utxo.0.value).sum::<u64>())
            .sum();
        debug!("Current balance: {} satoshis", balance);
        balance
    }
}
