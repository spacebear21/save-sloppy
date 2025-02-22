use anyhow::Result;
use axum::Router;
use clap::{Arg, Command};
use sloppy::Sloppy;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
pub mod nostr;
pub mod sloppy;
pub mod unleashed;

#[tokio::main]
async fn main() -> Result<()> {
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

    let app = Router::new().route("/", axum::routing::get(|| async { "Hello, World!" }));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    let server_task = tokio::spawn(async move { axum::serve(listener, app).await });

    let mut sloppy = Sloppy::new().await;
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
