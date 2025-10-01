use crate::serialization::Error as DeserializationError;
use bs58::decode::Error as Bs58Error;
use curv::elliptic::curves::{Ed25519, Point, Scalar};
use multi_party_eddsa::protocols::musig2::{PrivatePartialNonces, PublicPartialNonces};
use solana_client::client_error::ClientError;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use std::fmt::{Display, Formatter};
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    WrongNetwork(String),
    BadBase58(Bs58Error),
    WrongKeyPair(ed25519_dalek::SignatureError),
    AirdropFailed(ClientError),
    RecentHashFailed(ClientError),
    ConfirmingTransactionFailed(ClientError),
    BalaceFailed(ClientError),
    SendTransactionFailed(ClientError),
    DeserializationFailed {
        error: Box<DeserializationError>,
        field_name: &'static str,
    },
    MismatchMessages,
    InvalidSignature,
    KeyPairIsNotInKeys,
    InvalidPoint(curv::ErrorKey),
    InvalidScalar(curv::ErrorKey),
    BufferTooShort,
    InvalidPubkey,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongNetwork(net) => write!(
                f,
                "Unrecognized network: {}, please select Mainnet/Testnet/Devnet",
                net
            ),
            Self::BadBase58(e) => write!(f, "Based58 Error: {}", e),
            Self::WrongKeyPair(e) => write!(f, "Failed deserializing keypair: {}", e),
            Self::AirdropFailed(e) => write!(f, "Failed asking for an airdrop: {}", e),
            Self::RecentHashFailed(e) => write!(f, "Failed recieving the latest hash: {}", e),
            Self::ConfirmingTransactionFailed(e) => {
                write!(f, "Failed confirming transaction: {}", e)
            }
            Self::BalaceFailed(e) => write!(f, "Failed checking balance: {}", e),
            Self::SendTransactionFailed(e) => write!(f, "Failed sending transaction: {}", e),
            Self::DeserializationFailed { error, field_name } => {
                write!(f, "Failed deserializing {}: {}", field_name, error)
            }
            Self::MismatchMessages => write!(
                f,
                "There is a mismatch between first_messages and second_messages"
            ),
            Self::InvalidSignature => {
                write!(f, "The resulting signature doesn't match the transaction")
            }
            Self::KeyPairIsNotInKeys => {
                write!(f, "The provided keypair is not in the list of pubkeys")
            }
            Self::InvalidPoint(e) => write!(f, "Invalid point: {}", e),
            Self::InvalidScalar(e) => write!(f, "Invalid scalar: {}", e),
            Self::BufferTooShort => write!(f, "Buffer too short"),
            Self::InvalidPubkey => write!(f, "Invalid public key"),
        }
    }
}

impl From<Bs58Error> for Error {
    fn from(e: Bs58Error) -> Self {
        Self::BadBase58(e)
    }
}

impl From<ed25519_dalek::SignatureError> for Error {
    fn from(e: ed25519_dalek::SignatureError) -> Self {
        Self::WrongKeyPair(e)
    }
}

impl std::error::Error for Error {}

pub trait Serialize {
    fn serialize(&self, buffer: &mut Vec<u8>);
}

pub trait Deserialize: Sized {
    fn deserialize(buffer: &[u8]) -> Result<Self, Error>;
}

#[derive(Clone)]
pub struct AggMessage1 {
    pub sender: Pubkey,
    pub public_nonces: PublicPartialNonces,
}

impl Serialize for AggMessage1 {
    fn serialize(&self, buffer: &mut Vec<u8>) {
        buffer.extend_from_slice(&self.sender.to_bytes());

        // Serialize R values
        for r in &self.public_nonces.R {
            buffer.extend_from_slice(&r.to_bytes(true));
        }
    }
}

impl Deserialize for AggMessage1 {
    fn deserialize(buffer: &[u8]) -> Result<Self, Error> {
        if buffer.len() < 32 + 64 {
            return Err(Error::BufferTooShort);
        }

        let sender = Pubkey::new(&buffer[0..32]);

        let r1 = Point::from_bytes(&buffer[32..65]).map_err(Error::InvalidPoint)?;
        let r2 = Point::from_bytes(&buffer[65..98]).map_err(Error::InvalidPoint)?;

        let public_nonces = PublicPartialNonces { R: [r1, r2] };

        Ok(AggMessage1 {
            sender,
            public_nonces,
        })
    }
}

pub struct SecretAggStepOne {
    pub private_nonces: PrivatePartialNonces,
    pub public_nonces: PublicPartialNonces,
}

impl Serialize for SecretAggStepOne {
    fn serialize(&self, buffer: &mut Vec<u8>) {
        // Serialize private nonces
        for k in &self.private_nonces.k {
            buffer.extend_from_slice(&k.to_bytes());
        }

        // Serialize public nonces
        for r in &self.public_nonces.R {
            buffer.extend_from_slice(&r.to_bytes(true));
        }
    }
}

impl Deserialize for SecretAggStepOne {
    fn deserialize(buffer: &[u8]) -> Result<Self, Error> {
        if buffer.len() < 128 {
            return Err(Error::BufferTooShort);
        }

        let k1 = Scalar::from_bytes(&buffer[0..32]).map_err(Error::InvalidScalar)?;
        let k2 = Scalar::from_bytes(&buffer[32..64]).map_err(Error::InvalidScalar)?;

        let r1 = Point::from_bytes(&buffer[64..97]).map_err(Error::InvalidPoint)?;
        let r2 = Point::from_bytes(&buffer[97..130]).map_err(Error::InvalidPoint)?;

        Ok(SecretAggStepOne {
            private_nonces: PrivatePartialNonces { k: [k1, k2] },
            public_nonces: PublicPartialNonces { R: [r1, r2] },
        })
    }
}

#[derive(Clone)]
pub struct PartialSignature(pub Signature);

impl Serialize for PartialSignature {
    fn serialize(&self, buffer: &mut Vec<u8>) {
        buffer.extend_from_slice(&self.0.as_ref());
    }
}

impl Deserialize for PartialSignature {
    fn deserialize(buffer: &[u8]) -> Result<Self, Error> {
        if buffer.len() < 64 {
            return Err(Error::BufferTooShort);
        }

        let signature = Signature::new(&buffer[0..64]);
        Ok(PartialSignature(signature))
    }
}
