use uuid::Uuid;

use crate::U256;

pub struct Blockchain {
    blocks: Vec<Block>,
}

pub struct Block {
    header: BlockHeader,
    transactions: Vec<Transaction>,
}

pub struct BlockHeader {
    /// the time when the block was created. This is and the `nonce` are the two fields that alter
    /// when mining blocks in our blockchain.
    pub timestamp: u64,
    /// number only used once, we increment it to mine the block.
    pub nonce: u64,
    /// the hash of the previous block.
    pub prev_block_hash: [u8; 32],
    /// the hash of the Merkle tree root derived from all of the transactions (hashes) in this
    /// blocks. This ensure all transactions are accounted for and unalterable without changing the
    /// header. i.e Gather all transactions hashes of this header and digest in one master hash
    pub merkle_root: [u8; 32],
    /// a number, which has to be higher than the hash of this block for it to be considered valid.
    pub target: U256,
}

pub struct Transaction {
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
}

pub struct TransactionInput {
    /// the previous transaction output hash; Bitcoin uses the index of the output as well, we are
    /// gonna keep it simple for now.
    pub prev_transaction_output_hash: [u8; 32],
    /// this is how the user proves they can use the output of the previous transaction.
    /// in Bitcoin this would be the `script` field.
    pub signature: [u8; 64],
}

pub struct TransactionOutput {
    /// amount of currency being transferred in this output;
    pub value: u64,
    /// generated indentifier to help us ensure the transaction hash is unique.
    pub unique_id: Uuid,
    /// valid signature created with the private key
    pub pubkey: [u8; 33],
}

impl Blockchain {
    pub fn new() -> Self {
        Blockchain { blocks: vec![] }
    }

    pub fn add_block(&mut self, block: Block) {
        self.blocks.push(block);
    }
}

impl Block {
    pub fn new(header: BlockHeader, transactions: Vec<Transaction>) -> Self {
        Block {
            header,
            transactions,
        }
    }

    pub fn hash() -> ! {
        unimplemented!()
    }
}

impl BlockHeader {
    pub fn new(
        timestamp: u64,
        nonce: u64,
        prev_block_hash: [u8; 32],
        merkle_root: [u8; 32],
        target: U256,
    ) -> Self {
        BlockHeader {
            timestamp,
            nonce,
            prev_block_hash,
            merkle_root,
            target,
        }
    }

    pub fn hash() -> ! {
        unimplemented!()
    }
}

impl Transaction {
    pub fn new(inputs: Vec<TransactionInput>, outputs: Vec<TransactionOutput>) -> Self {
        Transaction { inputs, outputs }
    }

    pub fn hash() -> ! {
        unimplemented!()
    }
}
