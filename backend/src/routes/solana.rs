use actix_web::{HttpResponse, Result, web};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};

const RPC_URL: &str = "https://api.mainnet-beta.solana.com";
const JUP_QUOTE_API: &str = "https://quote-api.jup.ag/v6/quote";
const JUP_SWAP_API: &str = "https://quote-api.jup.ag/v6/swap";

#[derive(Deserialize)]
pub struct QuoteRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
}

#[derive(Serialize, Deserialize)]
pub struct QuoteResponse {
    pub in_amount: String,
    pub out_amount: String,
    pub other_amount_threshold: String,
    pub swap_mode: String,
    pub slippage_bps: u64,
}

#[derive(Deserialize)]
pub struct SwapRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
    pub user_pubkey: String,
}

#[derive(Serialize)]
pub struct SwapResponse {
    pub txid: String,
}

#[derive(Serialize)]
pub struct BalanceResponse {
    pub balance: u64,
}

#[derive(Serialize)]
pub struct TokenBalanceResponse {
    pub balance: u64,
}

#[actix_web::post("/quote")]
pub async fn quote(req: web::Json<QuoteRequest>) -> Result<HttpResponse> {
    let client = Client::new();
    let url = format!(
        "{}?inputMint={}&outputMint={}&amount={}&slippageBps=50",
        JUP_QUOTE_API, req.input_mint, req.output_mint, req.amount
    );

    let res = client
        .get(&url)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    Ok(HttpResponse::Ok().json(res))
}

#[actix_web::post("/swap")]
pub async fn swap(req: web::Json<SwapRequest>) -> Result<HttpResponse> {
    let client = Client::new();

    // Step 1: Fetch best route from Jupiter
    let quote_url = format!(
        "{}?inputMint={}&outputMint={}&amount={}&slippageBps=50",
        JUP_QUOTE_API, req.input_mint, req.output_mint, req.amount
    );
    let quote_res = client
        .get(&quote_url)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    // Step 2: Ask Jupiter to build the transaction
    let swap_tx = client
        .post(JUP_SWAP_API)
        .json(&serde_json::json!({
            "userPublicKey": req.user_pubkey,
            "quoteResponse": quote_res,
            "wrapAndUnwrapSol": true
        }))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    Ok(HttpResponse::Ok().json(swap_tx))
}

#[actix_web::get("/sol-balance/{pubkey}")]
pub async fn sol_balance() -> Result<HttpResponse> {
    let client = RpcClient::new(RPC_URL.to_string());
    let pubkey = Pubkey::from_str(&path.into_inner()).unwrap();
    let balance = client.get_balance(&pubkey).unwrap();
    Ok(HttpResponse::Ok().json(BalanceResponse { balance }))
}

#[actix_web::get("/token-balance/{pubkey}/{mint}")]
pub async fn token_balance() -> Result<HttpResponse> {
    let client = RpcClient::new(RPC_URL.to_string());
    let (pubkey_str, mint_str) = path.into_inner();
    let pubkey = Pubkey::from_str(&pubkey_str).unwrap();
    let mint = Pubkey::from_str(&mint_str).unwrap();

    let balances = client
        .get_token_accounts_by_owner(
            &pubkey,
            solana_client::rpc_client::TokenAccountsFilter::Mint(mint),
        )
        .unwrap();

    let balance = if let Some(account) = balances.value.first() {
        let data = &account.account.data;
        // decode SPL Token account data here...
        0u64
    } else {
        0u64
    };

    Ok(HttpResponse::Ok().json(TokenBalanceResponse { balance }))
}
