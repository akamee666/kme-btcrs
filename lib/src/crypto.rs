use ecdsa::{
    Signature as ECDSASignature, SigningKey, VerifyingKey, signature::SignerMut,
    signature::Verifier,
};
use k256::Secp256k1;
use serde::{Deserialize, Serialize};

use crate::sha256::Hash;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Signature(ECDSASignature<Secp256k1>);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PublicKey(VerifyingKey<Secp256k1>);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrivateKey(#[serde(with = "signkey_serde")] SigningKey<Secp256k1>);

mod signkey_serde {
    use serde::Deserialize;

    pub fn serialize<S>(
        key: &super::SigningKey<super::Secp256k1>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&key.to_bytes())
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<super::SigningKey<super::Secp256k1>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: Vec<u8> = Vec::<u8>::deserialize(deserializer)?;
        Ok(super::SigningKey::from_slice(&bytes).unwrap())
    }
}

impl PrivateKey {
    pub fn new_key() -> Self {
        PrivateKey(SigningKey::random(&mut rand::thread_rng()))
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey(*self.0.verifying_key())
    }
}

impl Signature {
    pub fn sign_output(output_hash: &Hash, private_key: &mut PrivateKey) -> Self {
        let signature = private_key.0.sign(&output_hash.as_bytes());
        Signature(signature)
    }

    pub fn verify(&self, output_hash: &Hash, public_key: &PublicKey) -> bool {
        public_key
            .0
            .verify(&output_hash.as_bytes(), &self.0)
            .is_ok()
    }
}
