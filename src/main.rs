use anyhow::Result;
use axum::Router;
use clap::{Arg, Command};
use sloppy::Sloppy;
use sqlx::pool::PoolConnection;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Connection, Database, Executor, Pool, Sqlite, SqliteConnection, SqlitePool};
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
pub mod nostr;
pub mod sloppy;
pub mod unleashed;

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // let matches = Command::new("saving-sloppy")
    //     .version("1.0")
    //     .arg(
    //         Arg::new("make-sloppy")
    //             .help("Creates sloppy")
    //             .long("make-sloppy")
    //             .action(clap::ArgAction::SetTrue),
    //     )
    //     .arg(
    //         Arg::new("post-to-nostr")
    //             .help("Do not do it, it doesn't work.")
    //             .long("post")
    //             .action(clap::ArgAction::SetTrue),
    //     )
    //     .get_matches();

    // if matches.get_flag("make-sloppy") {
    //     if let Err(e) = make_sloppy() {
    //         eprintln!("Error: {}", e);
    //     }
    // }

    // if matches.get_flag("post-to-nostr") {
    //     if let Err(e) = post_to_nostr() {
    //         eprintln!("Error: {}", e);
    //     }
    // }

    let db_filename = "db.sqlite";

    let pool: SqlitePool = SqlitePoolOptions::new().connect(db_filename).await?;

    let initialized_pool = InitializedPool::new(pool).await?;

    let app = Router::new().route("/", axum::routing::get(|| async { "Hello, World!" }));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    let server_task = tokio::spawn(async move { axum::serve(listener, app).await });

    let mut sloppy = Sloppy::new(initialized_pool.clone()).await;
    let sloppy_task = tokio::spawn(async move { sloppy.run_survival_loop().await });

    tokio::select! {
        result = server_task => {
            if let Err(e) = result {
                eprintln!("Server error: {}", e);
                std::process::exit(1);
            }
        }
        result = sloppy_task => {
            if let Err(e) = result {
                eprintln!("Sloppy error: {}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn make_sloppy() -> io::Result<()> {
    let log_file_path = "agent.log";
    let mut file = OpenOptions::new()
        .create(true) // Create the file if it does not exist
        .append(true) // Append to the file
        .open(log_file_path)?;

    writeln!(file, "Agent created at {}", chrono::Local::now())?;
    println!("Agent created and logged successfully.");
    Ok(())
}

fn save_to_log(item: &str) -> io::Result<()> {
    let log_file_path = "agent.log";
    let mut file = OpenOptions::new()
        .create(true) // Create the file if it does not exist
        .append(true) // Append to the file
        .open(log_file_path)?;

    writeln!(file, "{}", item)?;
    println!("item logged: {}", item);
    Ok(())
}

fn get_last_log_entry() -> io::Result<Option<String>> {
    let log_file_path = "agent.log";
    let file = File::open(log_file_path)?;
    let reader = BufReader::new(file);

    let last_line = reader.lines().filter_map(Result::ok).last();
    Ok(last_line)
}

fn save_to_file(filename: &str, content: &str) -> io::Result<()> {
    let mut file = File::create(filename)?;
    file.write_all(content.as_bytes())?;
    file.flush()?;
    Ok(())
}

fn read_from_file(filename: &str) -> io::Result<String> {
    let mut file = File::open(filename)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

#[derive(Clone)]
pub struct InitializedPool {
    pool: Pool<Sqlite>,
}

impl InitializedPool {
    #[tracing::instrument]
    pub async fn new(pool: Pool<Sqlite>) -> Result<InitializedPool> {
        // Initialize db tables
        let mut conn = pool.acquire().await?;

        // Using "query" (instead of `query!`) to avoid the sql checking that sqlx performs
        let x = sqlx::query(
        "CREATE TABLE IF NOT EXISTS campaign (id INTEGER PRIMARY KEY, text TEXT, created_at TEXT)",
        )
            .execute(conn.as_mut())
            .await?;
        info!("attempted to create table during pool initialization: {x:?}");

        let z = InitializedPool { pool };
        Ok(z)
    }
}

impl std::ops::Deref for InitializedPool {
    type Target = Pool<Sqlite>;
    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_hello_world() {
        insta::assert_debug_snapshot!(vec![1, 2, 3]);
    }
}
