use crate::{crypto::PublicKey, sha256::Hash, util::Saveable};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::crypto::Signature;
use std::io::{Error as IoError, ErrorKind as IoErrorKind, Read, Result as IoResult, Write};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transaction {
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionInput {
    /// the previous transaction output hash; Bitcoin uses the index of the output as well, we are
    /// gonna keep it simple for now.
    pub prev_transaction_output_hash: Hash,
    /// this is how the user proves they can use the output of the previous transaction.
    /// in Bitcoin this would be the `script` field.
    pub signature: Signature,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionOutput {
    /// amount of currency being transferred in this output;
    pub value: u64,
    /// generated indentifier to help us ensure the transaction hash is unique.
    pub unique_id: Uuid,
    /// valid signature created with the private key
    pub pubkey: PublicKey,
}

impl Transaction {
    pub fn new(inputs: Vec<TransactionInput>, outputs: Vec<TransactionOutput>) -> Self {
        Transaction { inputs, outputs }
    }

    pub fn hash(&self) -> Hash {
        Hash::hash(self)
    }
}

impl TransactionOutput {
    pub fn hash(&self) -> Hash {
        Hash::hash(self)
    }
}

impl Saveable for Transaction {
    fn load<I: Read>(reader: I) -> IoResult<Self> {
        ciborium::de::from_reader(reader).map_err(|_| {
            IoError::new(
                IoErrorKind::InvalidData,
                "Failed to deserialize Transaction",
            )
        })
    }
    fn save<O: Write>(&self, writer: O) -> IoResult<()> {
        ciborium::ser::into_writer(self, writer)
            .map_err(|_| IoError::new(IoErrorKind::InvalidData, "Failed to serialize Transaction"))
    }
}
