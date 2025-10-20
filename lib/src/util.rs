use serde::{Deserialize, Serialize};

use std::{
    fs::File,
    io::{Read, Result, Write},
    path::Path,
};

use crate::sha256::Hash;
use crate::types::Transaction;

#[derive(Debug, Clone, Deserialize, Serialize, Copy, Eq, PartialEq)]
pub struct MerkleRoot(Hash);

impl MerkleRoot {
    pub fn calculate(transactions: &[Transaction]) -> Self {
        let mut layer: Vec<Hash> = vec![];

        for transaction in transactions {
            layer.push(Hash::hash(transaction));
        }

        while layer.len() > 1 {
            let mut new_layer = vec![];
            for pair in layer.chunks(2) {
                let left = pair[0];
                let right = pair.get(1).unwrap_or(&pair[0]);
                new_layer.push(Hash::hash(&[left, *right]));
            }
            layer = new_layer
        }
        MerkleRoot(layer[0])
    }
}

pub trait Saveable
where
    Self: Sized,
{
    fn load<I: Read>(reader: I) -> Result<Self>;
    fn save<O: Write>(&self, writer: O) -> Result<()>;

    fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(&path)?;
        self.save(file)
    }

    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(&path)?;
        Self::load(file)
    }
}
