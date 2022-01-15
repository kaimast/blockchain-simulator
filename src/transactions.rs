use crate::crypto_helper::{AccountId, PublicKey, PrivateKey, to_account_id};
use serde::{Serialize, Deserialize};
use bytes::Bytes;
use std::fmt::Debug;

use sha2::Sha512;
use digest::Digest;

use rsa::hash::Hash;
use rsa::padding::PaddingScheme;

#[ derive(Serialize, Deserialize, Clone, Debug) ]
pub enum TxPayload<OpType> {
    CreateAccount { public_key: PublicKey },
    Operation { operation: OpType }
}

#[ derive(Serialize, Deserialize, Clone, Debug) ]
pub struct Transaction<OpType> {
    source: AccountId,
    payload: TxPayload<OpType>,
    signature: Bytes
}

impl<Operation: Serialize+Debug> Transaction<Operation> {
    pub fn new_create_account(public_key: PublicKey, sign_key: &PrivateKey) -> Self {
        let source = to_account_id(&public_key);
        let payload = TxPayload::CreateAccount{public_key};
        let data = bincode::serialize(&payload).unwrap();

        let mut hasher = Sha512::new();
        hasher.input(&data[..]);
        let hash = hasher.result();

        let sig = sign_key.sign(PaddingScheme::PKCS1v15Sign{hash: Some(Hash::SHA2_512)}, &hash).expect("sign payload");

        Self{ source, payload, signature: sig.into() }
    }

    pub fn new(source: AccountId, operation: Operation, sign_key: &PrivateKey) -> Self {
        let payload = TxPayload::Operation{operation};
        let data = bincode::serialize(&payload).unwrap();

        let mut hasher = Sha512::new();
        hasher.input(&data[..]);
        let hash = hasher.result();

        let sig = sign_key.sign(PaddingScheme::PKCS1v15Sign{hash: Some(Hash::SHA2_512)}, &hash).expect("sign payload");

        Self{ source, payload, signature: sig.into() }
    }

    pub fn get_source(&self) -> &AccountId {
        &self.source
    }

    pub fn get_payload(&self) -> &TxPayload<Operation> {
        &self.payload
    }

    pub fn into_payload(self) -> TxPayload<Operation> {
        self.payload
    }
}
