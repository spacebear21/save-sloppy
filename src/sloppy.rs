use anyhow::Result;
use bitcoin::Amount;
use nwc::prelude::*;
use rusqlite::{params, Row, ToSql};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Connection, FromRow, SqliteConnection};
use sqlx::{Pool, Sqlite};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

use crate::nostr::publish_on_nostr;
use crate::unleashed::{CampaignResponse, UnleashedClient};
use crate::{get_last_log_entry, read_from_file, save_to_file, save_to_log, InitializedPool};

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
    pool: InitializedPool,
}

impl Sloppy {
    pub async fn new(pool: InitializedPool) -> Self {
        // Initialize the AI agent
        Self {
            wallet: WalletState {
                onchain_balance: Amount::from_sat(0),
                lightning_balance: Amount::from_sat(0),
            },
            fundraising_history: Vec::new(),
            pool,
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

    // Get lightning balance using Nostr Wallet Connect
    async fn get_transactions(
        &mut self,
        nwc: &NWC,
    ) -> Result<Amount, Box<dyn Error + Send + Sync>> {
        let conn = rusqlite::Connection::open("sloppy")?;
        let create_txn_table = "
CREATE TABLE IF NOT EXISTS transaction (
  id               INTEGER PRIMARY KEY,
  type             TEXT CHECK(type IN ('incoming', 'outgoing'))
  invoice          TEXT
  description      TEXT
  description_hash TEXT
  preimage         TEXT
  payment_hash     TEXT NOT NULL
  amount_high      INTEGER NOT NULL
  amount_low       INTEGER NOT NULL
  fees_paid_high   INTEGER NOT NULL
  fees_paid_low    INTEGER NOT NULL
  created_at       TEXT NOT NULL
  expires_at       TEXT
  settled_at       TEXT
  metadata         TEXT
) ";
        conn.execute(create_txn_table, ())?;
        let params = params![];
        let insert_txn = "";

        let txns = nwc
            .list_transactions(ListTransactionsRequest::default())
            .await?;

        Ok(Amount::from_sat(100))
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

    async fn publish_post(&self, content: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        // Implement social media API integration
        let created_at = publish_on_nostr(content).await?;
        save_to_file("last_campaign", content)?;
        println!("Published post!");
        println!("{}", content);
        Ok(created_at)
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

        match nwc.get_info().await {
            Ok(resp) => println!("NWC: {:?}", resp),
            Err(err) => println!("NWC: {}", err),
        }

        // Generate initial fundraising post
        let post_content = self.generate_fundraising_post(&ai_client).await?;
        let post_content = remove_quotes(post_content.trim());

        // Publish post
        let created_at = self.publish_post(post_content).await?;

        let insert_sql = format!(
            "
        INSERT INTO campaigns ( text, created_at  )
        VALUES ( {post_content}, {created_at} )
        "
        );

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

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
struct SloppyLookupInvoiceResponse {
    pub inner: LookupInvoiceResponse,
}

impl SloppyLookupInvoiceResponse {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            inner: LookupInvoiceResponse {
                transaction_type: row
                    .get(0)
                    .ok()
                    .and_then(|s: String| serde_json::from_str(&s).unwrap()),
                invoice: row.get(1).ok(),
                description: row.get(2).ok(),
                description_hash: row.get(3).ok(),
                preimage: row.get(4).ok(),
                payment_hash: row.get(5)?,
                amount: ((row.get::<_, i64>(6)? as u64) << 32) | (row.get::<_, i64>(7)? as u64),
                fees_paid: ((row.get::<_, i64>(8)? as u64) << 32) | (row.get::<_, i64>(9)? as u64),
                created_at: row
                    .get(10)
                    .ok()
                    .and_then(|s: String| serde_json::from_str(&s).expect("created_at"))
                    .unwrap(),
                expires_at: row
                    .get(11)
                    .ok()
                    .and_then(|s: String| serde_json::from_str(&s).expect("expires_at")),
                settled_at: row
                    .get(12)
                    .ok()
                    .and_then(|s: String| serde_json::from_str(&s).expect("created_at"))
                    .unwrap(),
                metadata: row
                    .get::<_, Option<String>>(13)?
                    .map(|s| serde_json::from_str(&s).unwrap()),
            },
        })
    }
}

impl ToSql for SloppyLookupInvoiceResponse {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        let json_str = serde_json::to_string(self).map_err(|_| -> _ {
            rusqlite::Error::ToSqlConversionFailure("Failed to serialize to JSON".into())
        })?;
        Ok(rusqlite::types::ToSqlOutput::from(json_str))
    }
}

async fn get_campaign(pool: &sqlx::SqlitePool, id: i32) -> Result<Campaign, sqlx::Error> {
    let campaign = sqlx::query_as::<_, Campaign>("SELECT * FROM campaign WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await?;

    Ok(campaign)
}

async fn get_user(pool: &sqlx::SqlitePool, user_id: i32) -> Result<User, sqlx::Error> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;

    Ok(user)
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Campaign {
    pub id: i32,
    pub text: String,
    pub created_at: String,
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub created_at: String,
}
