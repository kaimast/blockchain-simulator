use rsa::PublicKeyParts;

use rand::rngs::OsRng;

use digest::Digest;
use sha2::Sha512;

pub type PublicKey = rsa::RsaPublicKey;
pub type PrivateKey = rsa::RsaPrivateKey;

// SHA-512 of the public key
pub type AccountId = u64;

pub fn generate_key_pair() -> (PrivateKey, PublicKey) {
    let mut rng = OsRng;
    let private = PrivateKey::new(&mut rng, 2048).unwrap();
    let public = private.to_public_key();

    (private, public)
}

pub fn to_account_id(key: &PublicKey) -> AccountId {
    let mut hasher = Sha512::new();
    
    let mut bytes = key.n().to_bytes_be();
    bytes.append(&mut key.e().to_bytes_be());

    hasher.input(&bytes);

    let output = hasher.result();
    let mut buffer: [u8; 8] = [0; 8];

    //FIXME get rid of this
    for i in 0..8 {
        buffer[i] = output[i];
    }

    unsafe {
        std::mem::transmute::<[u8; 8], u64>(buffer)
    }
}
