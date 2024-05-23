extern crate simple_log;
use simple_log::LogConfigBuilder;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::runtime::Runtime;
use google_cloud_pubsub::client::{Client, ClientConfig};
use google_cloud_pubsub::subscriber::ReceivedMessage;
use google_cloud_auth::credentials::CredentialsFile;

mod app_config;
pub use crate::app_config::lm_bot;

#[tokio::main]
async fn main() {
    setup_logging(lm_bot::SERVICE_NAME);
    log::info!("Starting Pub/Sub listener...");

    let running_flag = Arc::new(Mutex::new(true));
    let running_flag_clone = Arc::clone(&running_flag);

    tokio::spawn(async move {
        subscribe_to_pubsub(running_flag_clone).await;
    });

    // Keep the main thread alive while the listener is running
    while *running_flag.lock().unwrap() {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    log::info!("Pub/Sub listener stopped.");
}

fn setup_logging(service_name: &str) {
    let log_file = format!("{}.log", service_name);

    let config = LogConfigBuilder::builder()
        .path(log_file)
        .level("debug")
        .output_file()
        .output_console()
        .build();

    simple_log::new(config).unwrap();
}

async fn subscribe_to_pubsub(running_flag: Arc<Mutex<bool>>) {
    let cred = CredentialsFile::new_from_file("credentials.json".to_string()).await.unwrap();
    let client_config = ClientConfig::default().with_credentials(cred).await.unwrap();
    let client = Client::new(client_config).await.unwrap();
    let subscription = client.subscription("llm-prompt-sub");

    while *running_flag.lock().unwrap() {
        match subscription.pull(10, None).await {
            Ok(messages) => {
                for msg in messages {
                    print_message(&msg).await;
                    msg.ack().await.unwrap();
                }
            }
            Err(e) => {
                log::error!("Error pulling messages: {:?}", e);
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    }
}

async fn print_message(msg: &ReceivedMessage) {
    println!("Received message: {:?}", msg);
    log::info!("Received message: {:?}", msg);
}