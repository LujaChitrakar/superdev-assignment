use futures::StreamExt;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tokio::signal;
use tracing::{error, info, warn};
use yellowstone_grpc_proto::prelude::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterAccounts,
    SubscribeRequestFilterAccountsFilter, subscribe_update::UpdateOneof,
};
pub mod yellowstone;

#[derive(Debug, Clone)]
pub struct AccountUpdate {
    pub pubkey: String,
    pub lamports: u64,
    pub owner: String,
    pub executable: bool,
    pub rent_epoch: u64,
    pub data: Vec<u8>,
    pub write_version: u64,
    pub slot: u64,
}

pub struct AccountIndexer {
    client: GeyserGrpcClient<impl tonic::service::Interceptor>,
    accounts: HashMap<String, AccountUpdate>,
}

impl AccountIndexer {
    pub async fn new(
        endpoint: &str,
        token: Option<&str>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut builder = GeyserGrpcClient::build_from_shared(endpoint)?;

        if let Some(token) = token {
            builder = builder.x_token(Some(token))?;
        }

        let client = builder.connect().await?;

        Ok(Self {
            client,
            accounts: HashMap::new(),
        })
    }

    pub async fn index_accounts(
        &mut self,
        account_filters: Vec<AccountFilter>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!(
            "Starting account indexing with {} filters",
            account_filters.len()
        );

        // Create subscription request
        let mut accounts_filter = HashMap::new();

        for (index, filter) in account_filters.iter().enumerate() {
            let filter_key = format!("filter_{}", index);
            let account_filter = match filter {
                AccountFilter::Owner(owner) => SubscribeRequestFilterAccountsFilter {
                    owner: vec![owner.to_string()],
                    ..Default::default()
                },
                AccountFilter::Account(pubkey) => SubscribeRequestFilterAccountsFilter {
                    account: vec![pubkey.to_string()],
                    ..Default::default()
                },
                AccountFilter::ProgramData => SubscribeRequestFilterAccountsFilter {
                    owner: vec!["BPFLoaderUpgradeab1e11111111111111111111111".to_string()],
                    ..Default::default()
                },
                AccountFilter::TokenAccount => SubscribeRequestFilterAccountsFilter {
                    owner: vec!["TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string()],
                    ..Default::default()
                },
            };

            accounts_filter.insert(
                filter_key,
                SubscribeRequestFilterAccounts {
                    account: vec![account_filter],
                },
            );
        }

        let request = SubscribeRequest {
            accounts: accounts_filter,
            slots: HashMap::new(),
            transactions: HashMap::new(),
            transactions_status: HashMap::new(),
            blocks: HashMap::new(),
            blocks_meta: HashMap::new(),
            entry: HashMap::new(),
            commitment: Some(CommitmentLevel::Confirmed as i32),
            accounts_data_slice: vec![],
            ping: None,
        };

        info!("Subscribing to account updates...");
        let mut stream = self.client.subscribe_once(request).await?;

        // Handle updates
        while let Some(update) = stream.next().await {
            match update {
                Ok(msg) => {
                    if let Some(update_oneof) = msg.update_oneof {
                        self.handle_update(update_oneof).await;
                    }
                }
                Err(status) => {
                    error!("Stream error: {}", status);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_update(&mut self, update: UpdateOneof) {
        match update {
            UpdateOneof::Account(account_update) => {
                if let Some(account) = account_update.account {
                    let pubkey = bs58::encode(&account.pubkey).into_string();

                    let account_data = AccountUpdate {
                        pubkey: pubkey.clone(),
                        lamports: account.lamports,
                        owner: bs58::encode(&account.owner).into_string(),
                        executable: account.executable,
                        rent_epoch: account.rent_epoch,
                        data: account.data,
                        write_version: account.write_version,
                        slot: account_update.slot,
                    };

                    info!(
                        "Account update: {} (owner: {}, lamports: {})",
                        pubkey, account_data.owner, account_data.lamports
                    );

                    self.accounts.insert(pubkey, account_data);
                }
            }
            UpdateOneof::Slot(slot_update) => {
                info!(
                    "Slot update: {} (status: {:?})",
                    slot_update.slot, slot_update.status
                );
            }
            UpdateOneof::Transaction(tx_update) => {
                if let Some(transaction) = tx_update.transaction {
                    let signature = bs58::encode(&transaction.signature).into_string();
                    info!(
                        "Transaction update: {} (slot: {})",
                        signature, tx_update.slot
                    );
                }
            }
            _ => {
                // Handle other update types as needed
            }
        }
    }

    pub fn get_account(&self, pubkey: &str) -> Option<&AccountUpdate> {
        self.accounts.get(pubkey)
    }

    pub fn get_accounts_by_owner(&self, owner: &str) -> Vec<&AccountUpdate> {
        self.accounts
            .values()
            .filter(|account| account.owner == owner)
            .collect()
    }

    pub fn account_count(&self) -> usize {
        self.accounts.len()
    }

    pub async fn health_check(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let health_response = self.client.health_check().await?;
        info!("Health check: {:?}", health_response.status);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum AccountFilter {
    Owner(Pubkey),
    Account(Pubkey),
    ProgramData,
    TokenAccount,
}

#[tokio::main]
async fn main() {
    let endpoint = std::env::var("YELLOWSTONE_ENDPOINT")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com:443".to_string());
    let token = std::env::var("YELLOWSTONE_TOKEN").ok();

    let mut indexer = AccountIndexer::new(&endpoint, token.as_deref()).await?;

    // Health check
    match indexer.health_check().await {
        Ok(_) => info!("Connected to Yellowstone gRPC successfully"),
        Err(e) => {
            error!("Failed to connect: {}", e);
            return Err(e);
        }
    }

    let filters = vec![
        // Index all token accounts
        AccountFilter::TokenAccount,
        // Index a specific account (replace with actual pubkey)
        AccountFilter::Account(Pubkey::from_str("11111111111111111111111111111112")?),
        // Index accounts owned by System Program
        AccountFilter::Owner(Pubkey::from_str("11111111111111111111111111111111")?),
        // Index program data accounts
        AccountFilter::ProgramData,
    ];

    let shutdown = signal::ctrl_c();

    tokio::select! {
        result = indexer.index_accounts(filters) => {
            if let Err(e) = result {
                error!("Indexing error: {}", e);
            }
        }
        _ = shutdown => {
            info!("Received shutdown signal, stopping indexer...");
            info!("Indexed {} accounts", indexer.account_count());
        }
    }

    let client = GeyserGrpcClient::new(HealthClient::new(), GeyserClient::new());
    client.health_check().await;
}
