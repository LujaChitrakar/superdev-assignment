use actix_web::{App, Error, HttpResponse, HttpServer, Result, web::post};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    hash::Hash,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use std::str::FromStr;

pub mod error;
pub mod native_token;
pub mod serialization;
pub mod tss;

use crate::{
    serialization::{AggMessage1, Error, PartialSignature, SecretAggStepOne},
    tss::{key_agg, sign_and_broadcast, step_one, step_two},
};

#[derive(Deserialize)]
struct GenerateRequest {
    // No parameters needed for key generation
}

#[derive(Serialize)]
struct GenerateResponse {
    public_key: String,
    private_key: String,
}

#[derive(Deserialize)]
struct SendSingleRequest {
    private_key: String,
    to: String,
    amount: f64,
    memo: Option<String>,
    rpc_url: Option<String>,
}

#[derive(Serialize)]
struct SendSingleResponse {
    transaction_signature: String,
}

#[derive(Deserialize)]
struct AggregateKeysRequest {
    public_keys: Vec<String>,
    key_for_coefficient: Option<String>,
}

#[derive(Serialize)]
struct AggregateKeysResponse {
    aggregated_public_key: String,
}

#[derive(Deserialize)]
struct AggSendStep1Request {
    private_key: String,
}

#[derive(Serialize)]
struct AggSendStep1Response {
    message1: String,
    secret_state: String,
}

#[derive(Deserialize)]
struct AggSendStep2Request {
    private_key: String,
    amount: f64,
    to: String,
    memo: Option<String>,
    recent_block_hash: String,
    public_keys: Vec<String>,
    first_messages: Vec<String>, // Base64 encoded AggMessage1s
    secret_state: String,        // Base64 encoded SecretAggStepOne
}

#[derive(Serialize)]
struct AggSendStep2Response {
    partial_signature: String, // Base64 encoded PartialSignature
}

#[derive(Deserialize)]
struct AggregateSigsBroadcastRequest {
    amount: f64,
    to: String,
    memo: Option<String>,
    recent_block_hash: String,
    public_keys: Vec<String>,
    partial_signatures: Vec<String>, // Base64 encoded PartialSignatures
    rpc_url: Option<String>,
}

#[derive(Serialize)]
struct AggregateSigsBroadcastResponse {
    transaction_signature: String,
}

#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
    HttpServer::new(|| {
        App::new()
            .route("/generate", post().to(generate))
            .route("/send-single", post().to(send_single))
            .route("/aggregate-keys", post().to(aggregate_keys))
            .route("/agg-send-step1", post().to(agg_send_step1))
            .route("/agg-send-step2", post().to(agg_send_step2))
            .route(
                "/aggregate-signatures-broadcast",
                post().to(aggregate_signatures_broadcast),
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

async fn generate() -> Result<HttpResponse, Error> {
    let mut rng = rand::thread_rng();
    let keypair = Keypair::generate(&mut rng);
    let response = GenerateResponse {
        public_key: keypair.pubkey().to_string(),
        private_key: bs58::encode(keypair.to_bytes()).into_string(),
    };
    Ok(HttpResponse::Ok().body("Hello, world!"))
}

async fn send_single() -> Result<HttpResponse, Error> {
    let keypair_bytes = bs58::decode(&req.private_key)
        .into_vec()
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid private key: {}", e)))?;

    let keypair = Keypair::from_bytes(&keypair_bytes)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid keypair: {}", e)))?;

    let to_pubkey = Pubkey::from_str(&req.to).map_err(|e| {
        actix_web::error::ErrorBadRequest(format!("Invalid destination address: {}", e))
    })?;

    let rpc_url = req
        .rpc_url
        .as_deref()
        .unwrap_or("https://api.devnet.solana.com");
    let client = RpcClient::new(rpc_url);

    // Create transaction
    let lamports = sol_to_lamports(req.amount);
    let recent_blockhash = client.get_latest_blockhash().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to get recent blockhash: {}", e))
    })?;

    let mut transaction = Transaction::new_with_payer(
        &[system_instruction::transfer(
            &keypair.pubkey(),
            &to_pubkey,
            lamports,
        )],
        Some(&keypair.pubkey()),
    );

    transaction.sign(&[&keypair], recent_blockhash);

    let signature = client
        .send_and_confirm_transaction(&transaction)
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to send transaction: {}", e))
        })?;

    let response = SendSingleResponse {
        transaction_signature: signature.to_string(),
    };

    Ok(HttpResponse::Ok().body("Hello, world!"))
}

async fn aggregate_keys() -> Result<HttpResponse, Error> {
    let public_keys: Result<Vec<Pubkey>, _> = req
        .public_keys
        .iter()
        .map(|key_str| Pubkey::from_str(key_str))
        .collect();

    let public_keys = public_keys
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid public key: {}", e)))?;

    let key_for_coeff = req
        .key_for_coefficient
        .as_ref()
        .map(|key_str| Pubkey::from_str(key_str))
        .transpose()
        .map_err(|e| {
            actix_web::error::ErrorBadRequest(format!("Invalid coefficient key: {}", e))
        })?;

    let agg_key = key_agg(public_keys, key_for_coeff)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Key aggregation failed: {}", e)))?;

    let agg_pubkey = Pubkey::new(&*agg_key.agg_public_key.to_bytes(true));

    let response = AggregateKeysResponse {
        aggregated_public_key: agg_pubkey.to_string(),
    };

    Ok(HttpResponse::Ok().body("Hello, world!"))
}

async fn agg_send_step1() -> Result<HttpResponse, Error> {
    let keypair_bytes = bs58::decode(&req.private_key)
        .into_vec()
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid private key: {}", e)))?;

    let keypair = Keypair::from_bytes(&keypair_bytes)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid keypair: {}", e)))?;

    let (message1, secret_state) = step_one(keypair);

    let mut msg1_bytes = Vec::new();
    message1.serialize(&mut msg1_bytes);

    let mut secret_bytes = Vec::new();
    secret_state.serialize(&mut secret_bytes);

    let response = AggSendStep1Response {
        message1: base64::encode(msg1_bytes),
        secret_state: base64::encode(secret_bytes),
    };
    Ok(HttpResponse::Ok().body("Hello, world!"))
}

async fn agg_send_step2() -> Result<HttpResponse, Error> {
    let keypair_bytes = bs58::decode(&req.private_key)
        .into_vec()
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid private key: {}", e)))?;

    let keypair = Keypair::from_bytes(&keypair_bytes)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid keypair: {}", e)))?;

    let to_pubkey = Pubkey::from_str(&req.to).map_err(|e| {
        actix_web::error::ErrorBadRequest(format!("Invalid destination address: {}", e))
    })?;

    let recent_block_hash = Hash::from_str(&req.recent_block_hash)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid block hash: {}", e)))?;

    let public_keys: Result<Vec<Pubkey>, _> = req
        .public_keys
        .iter()
        .map(|key_str| Pubkey::from_str(key_str))
        .collect();
    let public_keys = public_keys
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid public key: {}", e)))?;

    // Deserialize first messages
    let first_messages: Result<Vec<AggMessage1>, _> = req
        .first_messages
        .iter()
        .map(|msg_str| {
            let bytes =
                base64::decode(msg_str).map_err(|e| format!("Base64 decode error: {}", e))?;
            AggMessage1::deserialize(&bytes).map_err(|e| format!("Deserialization error: {}", e))
        })
        .collect();
    let first_messages = first_messages.map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    // Deserialize secret state
    let secret_bytes = base64::decode(&req.secret_state)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid secret state: {}", e)))?;
    let secret_state = SecretAggStepOne::deserialize(&secret_bytes)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid secret state: {}", e)))?;

    let partial_sig = step_two(
        keypair,
        req.amount,
        to_pubkey,
        req.memo.clone(),
        recent_block_hash,
        public_keys,
        first_messages,
        secret_state,
    )
    .map_err(|e| actix_web::error::ErrorBadRequest(format!("Step 2 failed: {}", e)))?;

    let mut sig_bytes = Vec::new();
    partial_sig.serialize(&mut sig_bytes);

    let response = AggSendStep2Response {
        partial_signature: base64::encode(sig_bytes),
    };
    Ok(HttpResponse::Ok().body("Hello, world!"))
}

async fn aggregate_signatures_broadcast() -> Result<HttpResponse, Error> {
    let to_pubkey = Pubkey::from_str(&req.to).map_err(|e| {
        actix_web::error::ErrorBadRequest(format!("Invalid destination address: {}", e))
    })?;

    let recent_block_hash = Hash::from_str(&req.recent_block_hash)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid block hash: {}", e)))?;

    let public_keys: Result<Vec<Pubkey>, _> = req
        .public_keys
        .iter()
        .map(|key_str| Pubkey::from_str(key_str))
        .collect();
    let public_keys = public_keys
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid public key: {}", e)))?;

    // Deserialize partial signatures
    let partial_signatures: Result<Vec<PartialSignature>, _> = req
        .partial_signatures
        .iter()
        .map(|sig_str| {
            let bytes =
                base64::decode(sig_str).map_err(|e| format!("Base64 decode error: {}", e))?;
            PartialSignature::deserialize(&bytes)
                .map_err(|e| format!("Deserialization error: {}", e))
        })
        .collect();
    let partial_signatures =
        partial_signatures.map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    let transaction = sign_and_broadcast(
        req.amount,
        to_pubkey,
        req.memo.clone(),
        recent_block_hash,
        public_keys,
        partial_signatures,
    )
    .map_err(|e| actix_web::error::ErrorBadRequest(format!("Aggregation failed: {}", e)))?;

    let rpc_url = req
        .rpc_url
        .as_deref()
        .unwrap_or("https://api.devnet.solana.com");
    let client = RpcClient::new(rpc_url);

    let signature = client
        .send_and_confirm_transaction(&transaction)
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to send transaction: {}", e))
        })?;

    let response = AggregateSigsBroadcastResponse {
        transaction_signature: signature.to_string(),
    };

    Ok(HttpResponse::Ok().body("Hello, world!"))
}

fn sol_to_lamports(sol: f64) -> u64 {
    (sol * 1_000_000_000.0) as u64
}
