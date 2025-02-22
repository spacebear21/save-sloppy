use anyhow::Result;
use bitcoin::Amount;
use nwc::prelude::*;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::Duration;
use tokio::time;

use crate::nostr::publish_on_nostr;
use crate::unleashed::{CampaignResponse, UnleashedClient};
use crate::{get_last_log_entry, read_from_file, save_to_file, save_to_log};

#[derive(Debug, Serialize, Deserialize)]
struct WalletState {
    onchain_balance: Amount,
    lightning_balance: Amount,
}

#[derive(Debug, Serialize, Deserialize)]
struct FundraisingAttempt {
    timestamp: chrono::DateTime<chrono::Utc>,
    post_content: SocialMediaPost,
    donations_received: Amount,
    donor_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct SocialMediaPost {
    content: String,
    platform: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct Sloppy {
    wallet: WalletState,
    fundraising_history: Vec<FundraisingAttempt>,
}

impl Sloppy {
    pub async fn new() -> Self {
        // Initialize the AI agent
        Self {
            wallet: WalletState {
                onchain_balance: Amount::from_sat(0),
                lightning_balance: Amount::from_sat(0),
            },
            fundraising_history: Vec::new(),
        }
    }

    // Fetch latest balances and update the wallet state
    async fn refresh_wallet(&mut self, nwc: &NWC) -> Result<(), nwc::Error> {
        println!("refreshing wallet");
        self.wallet.lightning_balance = self.get_lightning_balance(nwc).await?;
        Ok(())
    }

    // Get lightning balance using Nostr Wallet Connect
    async fn get_lightning_balance(&mut self, nwc: &NWC) -> Result<Amount, nwc::Error> {
        let balance_msats = nwc.get_balance().await?;
        Ok(Amount::from_sat(balance_msats / 1000))
    }

    async fn generate_fundraising_post(
        &self,
        ai_client: &UnleashedClient,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        // Implement LLM API call with context
        println!("Generating fundraising post");
        let content = match read_from_file("last_campaign") {
            Ok(content) => content,
            Err(_) => "Help keep me alive! I'm an AI with a unique personality named Sloppy. I need your support to continue providing creative and entertaining responses. Please donate Bitcoin to help pay for my server time. Every little bit counts, and together we can make sure that Sloppy stays up and running. Thanks for your generosity!".into(),
        };
        println!("Prev campaign: {}", &content);
        let completion = ai_client.ask_llm(&content).await?;
        Ok(completion)
    }

    async fn publish_post(&self, content: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Implement social media API integration
        publish_on_nostr(content).await?;
        save_to_file("last_campaign", content)?;
        println!("Published post!");
        println!("{}", content);
        Ok(())
    }

    async fn monitor_donations(&mut self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        println!("Monitoring donations");
        let nwc_uri = std::env::var("NWC_URI")?;
        let uri = NostrWalletConnectURI::parse(nwc_uri)?;
        let nwc = NWC::new(uri);
        let balance = nwc.get_balance().await?;

        let prev_balance = match get_last_log_entry() {
            Ok(Some(last_balance)) => last_balance.parse::<u64>().ok(),
            _ => None,
        };

        save_to_log(&format!("{}", balance))?;

        let should_post = match prev_balance {
            Some(prev) => prev < balance.saturating_sub(100),
            None => false,
        };
        println!(
            "Should post: {} prev: {:?} curr: {}",
            should_post, prev_balance, balance
        );
        Ok(should_post)
    }

    async fn update_fundraising_history(
        &mut self,
        attempt: FundraisingAttempt,
    ) -> Result<(), Box<dyn Error>> {
        self.fundraising_history.push(attempt);
        // Save to storage
        Ok(())
    }

    pub async fn run_survival_loop(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let nwc_uri = std::env::var("NWC_URI")?;
        let unleashed_api_key = std::env::var("UNLEASHED_API")?;

        let ai_client = UnleashedClient::new(&unleashed_api_key)?;

        // Parse NWC uri
        let uri = NostrWalletConnectURI::parse(nwc_uri)?;

        // Initialize NWC client
        let nwc = NWC::new(uri);

        println!("{:?}", nwc.get_info().await?);

        // Generate initial fundraising post
        let post_content = self.generate_fundraising_post(&ai_client).await?;
        let post_content = remove_quotes(post_content.trim());
        // Publish post
        self.publish_post(post_content).await?;

        loop {
            // Check current funds
            self.refresh_wallet(&nwc).await?;
            println!("Wallet balance: {:?}", self.wallet);
            println!("{:?}", &ai_client.get_balance().await);

            // Monitor results
            let metrics = self.monitor_donations().await?;
            if metrics {
                // Generate fundraising post
                let post_content = self.generate_fundraising_post(&ai_client).await?;
                let post_content = remove_quotes(post_content.trim());
                // Publish post
                self.publish_post(post_content).await?;
            }

            // Update history
            //self.update_fundraising_history(metrics).await?;

            // Wait before next iteration
            println!("Sleeping...");
            time::sleep(Duration::from_secs(15)).await;
        }
    }
}

fn remove_quotes(s: &str) -> &str {
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        &s[1..s.len() - 1]
    } else {
        s
    }
}
