use crate::yellowstone::GeyserGrpcClient;

pub mod yellowstone;

#[tokio::main]
async fn main() {   
    // Replace with your Geyser gRPC endpoint
    let endpoint = "https://mainnet.rpc.solana.com:443"; 
    let x_token: Option<String> = None; // Replace with your token if needed

    let mut client = GeyserGrpcClient::build_from_shared(endpoint)
        .unwrap()
        .x_token(x_token)
        .unwrap()
        .connect_lazy()
        .unwrap();

    match client.health_check().await {
        Ok(response) => println!("Health check response: {:?}", response),
        Err(e) => eprintln!("Health check failed: {}", e),
    }
}