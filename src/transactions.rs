use std::fmt::Debug;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use rsa::pss::SigningKey;
use rsa::signature::{SignatureEncoding, RandomizedSigner};
use sha2::{Digest, Sha256, Sha512};

use crate::crypto_helper::{to_account_id, AccountId, PrivateKey, PublicKey};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TxPayload<OpType> {
    CreateAccount { public_key: PublicKey },
    Operation { operation: OpType },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transaction<OpType> {
    source: AccountId,
    payload: TxPayload<OpType>,
    signature: Bytes,
}

impl<Operation: Serialize + Debug> Transaction<Operation> {
    pub fn new_create_account(public_key: PublicKey, private_key: PrivateKey) -> Self {
        let source = to_account_id(&public_key);
        let payload = TxPayload::CreateAccount { public_key };
        let data = bincode::serialize(&payload).unwrap();

        let mut hasher = Sha512::new();
        hasher.update(&data[..]);
        let hash = hasher.finalize();

        let mut rng = rand::thread_rng();
        let signing_key = SigningKey::<Sha256>::new(private_key);
        let sig = signing_key.sign_with_rng(&mut rng, &hash);

        Self {
            source,
            payload,
            signature: sig.to_vec().into(),
        }
    }

    pub fn new(source: AccountId, operation: Operation, private_key: PrivateKey) -> Self {
        let payload = TxPayload::Operation { operation };
        let data = bincode::serialize(&payload).unwrap();

        let mut hasher = Sha512::new();
        hasher.update(&data[..]);
        let hash = hasher.finalize();

        let mut rng = rand::thread_rng();
        let signing_key = SigningKey::<Sha256>::new(private_key);
        let sig = signing_key.sign_with_rng(&mut rng, &hash);

        Self {
            source,
            payload,
            signature: sig.to_vec().into(),
        }
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
