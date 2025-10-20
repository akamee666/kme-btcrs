use crate::{
    crypto::PublicKey,
    types::{Block, Transaction, TransactionOutput},
};

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
