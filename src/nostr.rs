use nostr_sdk::prelude::*;
use std::error::Error;

pub async fn publish_on_nostr(note: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    // Or use your already existing (from hex or bech32)
    let nostr_seckey = std::env::var("NOSTR_SECKEY")?;
    let keys = Keys::parse(&nostr_seckey)?;

    // Show bech32 public key
    let bech32_pubkey: String = keys.public_key().to_bech32()?;
    println!("Bech32 PubKey: {}", bech32_pubkey);

    // Configure client to use proxy for `.onion` relays
    //let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 9050));
    let connection: Connection = Connection::new();
    let opts = Options::new().connection(connection);

    // Create new client with custom options
    let client = Client::builder().signer(keys.clone()).opts(opts).build();

    // Add relays
    client.add_relay("wss://relay.damus.io").await?;
    client.add_relay("wss://nos.lol").await?;

    // Add read relay
    client.add_read_relay("wss://relay.nostr.info").await?;

    // Connect to relays
    client.connect().await;

    let nostr_name = std::env::var("NOSTR_NAME")?;
    let nostr_display_name = std::env::var("NOSTR_DISPLAY_NAME")?;
    let nostr_about = std::env::var("NOSTR_ABOUT")?;
    let nostr_picture = std::env::var("NOSTR_PICTURE")?;
    let nostr_banner = std::env::var("NOSTR_BANNER")?;
    let nostr_nip_05 = std::env::var("NOSTR_NIP_05")?;
    let nostr_lud_16 = std::env::var("NOSTR_LUD_16")?;

    let metadata = Metadata::new()
        .name(nostr_name)
        .display_name(nostr_display_name)
        .about(nostr_about)
        .picture(Url::parse(&nostr_picture)?)
        .banner(Url::parse(&nostr_banner)?)
        .nip05(nostr_nip_05)
        .lud16(nostr_lud_16);

    // Update metadata
    client.set_metadata(&metadata).await?;

    // Publish a text note

    // let builder = EventBuilder::text_note("Yet another post from rust-nostr!");
    // client.send_event_builder(builder).await?;

    // Create a POW text note
    let builder = EventBuilder::text_note(note).pow(20);
    client.send_event_builder(builder).await?; // Send to all relays
                                               // client.send_event_builder_to(["wss://relay.damus.io"], builder).await?; // Send to specific relay
    let created_at = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();
    let created_at = format!("{}", created_at);

    Ok(created_at)
}
