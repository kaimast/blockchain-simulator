use rsa::PublicKey as PubKey;
use rand::rngs::OsRng;

use digest::Digest;
use sha2::Sha512;

pub type PublicKey = rsa::RSAPublicKey;
pub type PrivateKey = rsa::RSAPrivateKey;

// SHA-512 of the public key
pub type AccountId = [u8; 8]; //GenericArray::<u8, 8>;

pub fn generate_key_pair() -> (PrivateKey, PublicKey) {
    let mut rng = OsRng;
    let private = PrivateKey::new(&mut rng, 2048).unwrap();
    let public = private.to_public_key();

    return (private, public);
}

pub fn to_account_id(key: &PublicKey) -> AccountId {
    let mut hasher = Sha512::new();
    
    let mut bytes = key.n().to_bytes_be();
    bytes.append(&mut key.e().to_bytes_be());

    hasher.input(&bytes);

    let output = hasher.result();
    let mut result : AccountId = [0; 8];

    for i in 0..8 {
        result[i] = output[i];
    }

    return result;
}
