use crate::{
    crypto::PublicKey,
    types::{Block, Transaction, TransactionOutput},
};

use std::io::{Error as IoError, Read, Write};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Serialize, Deserialize)]
pub enum Message {
    /// Fetch all UTXOs belonging to a owner/wallet/public key. That's how we are going to know how
    /// much satoshis we have
    FetchUTXOS(PublicKey),
    /// UTXOs belonging to a public key. Bool determines if marked (Already spent)
    UTXOs(Vec<(bool, TransactionOutput)>),
    /// Send a transaction to the network.
    SubmitTransaction(Transaction),
    /// Broadcast a new transaction to other nodes
    NewTransaction(Transaction),
    /// Ask the node to prepare the optimal block template with the coinbase transaction paying the
    /// specified public key (e.g block mined i guess)
    FetchTemplate(PublicKey),
    /// The template of a block
    Template(Block),
    /// Ask the node to validate a block template.
    /// this is to prevent the node from mining an invalid block (e.g if one has been found in the
    /// meantime, or if transactions have been removed from the mempool)
    ValidateTemplate(Block),
    /// if template is valid
    TemplateValidity(bool),
    /// Submit a mined block to a node
    SubmitTemplate(Block),
    /// Ask a node to report all the other nodes it knows about
    DiscoverNodes,
    /// Response to DiscoverNodes
    NodeList(Vec<String>),
    /// Ask a node whats the highest block it knows about in comparison to the local blockchain
    AskDifference(i32),
    /// Response to AskDifference
    Difference(i32),
    /// Ask a node to send a block with the specified height
    FetchBlock(usize),
    /// Broadcast a new block to other nodes
    NewBlock(Block),
}

impl Message {
    pub fn encode(&self) -> Result<Vec<u8>, ciborium::ser::Error<IoError>> {
        let mut bytes = Vec::new();
        ciborium::into_writer(self, &mut bytes)?;
        Ok(bytes)
    }

    pub fn decode(data: &[u8]) -> Result<Self, ciborium::de::Error<IoError>> {
        ciborium::from_reader(data)
    }

    pub fn send(&self, stream: &mut impl Write) -> Result<(), ciborium::ser::Error<IoError>> {
        let bytes = self.encode()?;
        let len = bytes.len() as u64;
        stream.write_all(&len.to_be_bytes())?;
        // shouldn't we check if receiver received the len first?
        stream.write_all(&bytes)?;
        Ok(())
    }

    pub fn receive(stream: &mut impl Read) -> Result<Self, ciborium::de::Error<IoError>> {
        let mut len_bytes = [0u8; 8];
        stream.read_exact(&mut len_bytes)?;
        let len = u64::from_be_bytes(len_bytes) as usize;
        let mut data = vec![0u8; len];
        stream.read_exact(&mut data)?;
        Self::decode(&data)
    }

    pub async fn send_async(
        &self,
        stream: &mut (impl AsyncWrite + Unpin),
    ) -> Result<(), ciborium::ser::Error<IoError>> {
        let bytes = self.encode()?;
        let len = bytes.len() as u64;
        stream.write_all(&len.to_be_bytes()).await?;
        // shouldn't we check if receiver received the len first?
        stream.write_all(&bytes).await?;
        Ok(())
    }

    pub async fn receive_async(
        stream: &mut (impl AsyncRead + Unpin),
    ) -> Result<Self, ciborium::de::Error<IoError>> {
        let mut len_bytes = [0u8; 8];
        stream.read_exact(&mut len_bytes).await?;
        let len = u64::from_be_bytes(len_bytes) as usize;
        let mut data = vec![0u8; len];
        stream.read_exact(&mut data).await?;
        Self::decode(&data)
    }
}
