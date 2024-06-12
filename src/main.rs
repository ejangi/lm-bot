#[macro_use]
extern crate windows_service;

#[macro_use]
extern crate log;

use std::ffi::OsString;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_dispatcher;
use tokio::runtime::Runtime;
use google_cloud_pubsub::client::{Client, ClientConfig};
use google_cloud_pubsub::subscriber::ReceivedMessage;
use google_cloud_auth::credentials::CredentialsFile;
mod app_config;

pub use crate::app_config::lm_bot;

define_windows_service!(ffi_service_main, my_service_main);

fn my_service_main(arguments: Vec<OsString>) {
    info!("entered my_service_main()");

    if let Err(e) = run_service(arguments) {
        eprintln!("Service error: {:?}", e);
        log::error!("Service error: {:?}", e);
    }
}

fn run_service(_arguments: Vec<OsString>) -> windows_service::Result<()> {
    eventlog::init(lm_bot::SERVICE_NAME, log::Level::Info).unwrap();
    info!("entered run_service()");

    let running_flag = Arc::new(Mutex::new(true));
    let running_flag_clone = Arc::clone(&running_flag);
    let running_flag_clone2 = Arc::clone(&running_flag);

    // Register the service control handler
    let status_handle = service_control_handler::register(lm_bot::SERVICE_NAME, move |control_event| {
        match control_event {
            ServiceControl::Stop => {
                let mut running = running_flag.lock().unwrap();
                *running = false;
                ServiceControlHandlerResult::NoError
            }

            ServiceControl::UserEvent(code) => {
                if code.to_raw() == 130 {
                    let mut running = running_flag.lock().unwrap();
                    *running = false;
                }
                ServiceControlHandlerResult::NoError
            }

            _ => ServiceControlHandlerResult::NotImplemented,
        }
    })?;

    // Report the running status
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    info!("Service running");

    // Run the Pub/Sub subscription in a Tokio runtime
    thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            subscribe_to_pubsub(running_flag_clone).await;
        });
    });

    // Wait for the stop signal
    while *running_flag_clone2.lock().unwrap() {
        thread::sleep(Duration::from_secs(1));
    }

    // Report the stopped status
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    info!("Service stopping");

    Ok(())
}

async fn subscribe_to_pubsub(running_flag: Arc<Mutex<bool>>) {
    let cred_string = include_str!("credentials.json");
    let cred = match CredentialsFile::new_from_str(&cred_string).await {
        Ok(value) => value,
        Err(err) => {
            error!("Could not get credentials from file {}", err);
            CredentialsFile::new().await.unwrap()
        }
    };

    if cred.client_email.is_none() {
        error!("No client_email");
    }

    let client_config = match ClientConfig::default().with_credentials(cred).await {
        Ok(value) => value,
        Err(err) => {
            error!("Error creating ClientConfig.with_credentials(): {}", err);
            ClientConfig::default()
        }
    };

    let client = match Client::new(client_config).await {
        Ok(value) => value,
        Err(err) => {
            error!("Unable to initialise Client struct. {}", err);
            return
        }
    };

    let subscription = client.subscription("llm-prompt-sub");
    match subscription.exists(None).await {
        Ok(value) => info!("Subscription exists: {}", value),
        Err(err) => error!("Subscription does not exist {}", err)
    }

    info!("Subscription ready. Starting loop...");

    while *running_flag.lock().unwrap() {
        let message: Vec<ReceivedMessage> = subscription.pull(10, None).await.unwrap();
        for msg in message {
            print_message(&msg.message.data).await;
            msg.ack().await.unwrap();
        }
    }
}

async fn print_message(msg: &Vec<u8>) {
    match String::from_utf8(msg.to_owned()){
        Ok(value) => {
            info!("Received message: {:?}", value);
            value
        }
        Err(err) => {
            error!("Unable to convert Vec<u8> message to string. {}", err);
            "".to_string()
        }
    };
}

fn main() -> windows_service::Result<()> {
    service_dispatcher::start(lm_bot::SERVICE_NAME, ffi_service_main)?;
    Ok(())
}
