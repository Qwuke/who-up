use figment::providers::Format;
use rocket::tokio::time::{sleep, Duration};
use rocket::State;
use figment::{Figment, providers::Toml};
use serde::Deserialize;
use std::future::IntoFuture;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use twilight_http::Client;
use twilight_model::id::{Id, marker::{MessageMarker, ChannelMarker}};

const HOUR_INCREMENT_IN_MS: u64 = 3600000;
const DISCORD_MESSAGE_PREFIX: &str = "HackManhattan will have people in it for the next";
const DISCORD_MESSAGE_EMPTY: &str = "There is no one in HackManhattan 😴";

#[macro_use] extern crate rocket;

#[derive(Deserialize)]
struct Config {
    discord_token: String,
    discord_channel: String,
}

struct TimerCounter {
    pub count: AtomicU64,
    pub discord_message_id: Id<MessageMarker>,
    pub discord_channel_id: Id<ChannelMarker>,
    pub discord_client: Client,
}

#[post("/timer")]
fn add_timer(timer_count: &State<Arc<TimerCounter>>) -> String {
    // Ordering can be relaxed in all operations - our commutative operations allow for correctness in any order
    let mut count = timer_count.count.fetch_add(HOUR_INCREMENT_IN_MS, Ordering::Relaxed) + HOUR_INCREMENT_IN_MS;
    if count > HOUR_INCREMENT_IN_MS * 6 {
        timer_count.count.swap(0, Ordering::Relaxed);
        count = 0;
    }
    format!("{}", count)
}

#[get("/timer")]
fn timer(timer_count: &State<Arc<TimerCounter>>) -> String {
    format!("{}", timer_count.count.load(Ordering::Relaxed))
}

async fn timer_loop(context: Arc<TimerCounter>) {
    let interval = Duration::from_secs(10);

    loop {
        sleep(interval).await;
        // Get timer state
        let mut remaining_time = Duration::new(0, 0);
        if context.count.load(Ordering::Relaxed) > 0 {
            remaining_time = Duration::from_millis(
                context.count.fetch_sub(interval.as_secs() * 1000, Ordering::Relaxed) - interval.as_secs() * 1000
            );   
        }

        // Call discord message update, and update how many hours + minutes are left on the timer
        let minutes = (remaining_time.as_secs() / 60) % 60;
        let hours = (remaining_time.as_secs() / 60) / 60;
        let discord_message; 
        if hours > 0 {
            discord_message = format!("{} {} hours and {} minutes 🤠", DISCORD_MESSAGE_PREFIX, hours, minutes);
        } else if minutes > 0  {
            discord_message = format!("{} {} minutes 🤠", DISCORD_MESSAGE_PREFIX, minutes);
        } else {
            discord_message = DISCORD_MESSAGE_EMPTY.to_string();
        }
        
        // Update bot discord message in channel
        if let Err(err) = context.discord_client.update_message(context.discord_channel_id, context.discord_message_id)
            .content(Some(&discord_message)).unwrap().into_future().await {
                eprintln!("There was an error updating the message! {}", err)
        };
    }
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let config: Config = Figment::from(Toml::file("Secrets.toml")).extract().expect("Config should be extractable");
    let discord_client = Client::new(config.discord_token);
    let channel_id = Id::new(config.discord_channel.parse().expect("Config file should be parsable"));

    // Create message in discord channel
    discord_client.create_message(channel_id)
        .content(DISCORD_MESSAGE_EMPTY)
        .expect("There was an issue creating the initial message request")
        .into_future().await
        .expect("Initial message should have sent");
    
    let messages = discord_client.channel_messages(channel_id)
        .limit(1).expect("Channel message request should have been created")
        .await.expect("Channel messages should be retrievable")
        .models()
        .await.expect("Channel messages should be convertable");

    let message = messages.first().expect("Should find at least one channel message");
        
    let context = Arc::new(TimerCounter { 
        count: AtomicU64::new(0),
        discord_client: discord_client,
        discord_message_id: message.id,
        discord_channel_id: channel_id,
    });
    
    // Configure rocket with thread-safe state
    let r = rocket::build()
        .manage(context.clone())
        .mount("/", routes![add_timer, timer])
        .ignite().await?;

    // Start countdown timer
    rocket::tokio::task::spawn(async move {
        timer_loop(context).await
    });

    // Start server
    r.launch().await?; 

    Ok(())
}
