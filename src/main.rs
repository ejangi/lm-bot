extern crate simple_log;
use simple_log::LogConfigBuilder;
use std::ffi::OsString;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use windows_service::define_windows_service;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_dispatcher;
use tokio::runtime::Runtime;
use google_cloud_pubsub::client::{Client, ClientConfig};
use google_cloud_pubsub::subscriber::ReceivedMessage;
use google_cloud_auth::credentials::CredentialsFile;
extern crate directories;
use directories::BaseDirs;
use std::path::Path;
use std::io;
use std::fs::OpenOptions;

define_windows_service!(ffi_service_main, my_service_main);

fn my_service_main(arguments: Vec<OsString>) {
    if let Err(e) = run_service(arguments) {
        eprintln!("Service error: {:?}", e);
        log::error!("Service error: {:?}", e);
    }
}

fn run_service(_arguments: Vec<OsString>) -> windows_service::Result<()> {
    let running_flag = Arc::new(Mutex::new(true));
    let running_flag_clone = Arc::clone(&running_flag);
    let running_flag_clone2 = Arc::clone(&running_flag);

    // Register the service control handler
    let status_handle = service_control_handler::register("lm-bot", move |control_event| {
        match control_event {
            ServiceControl::Stop => {
                let mut running = running_flag.lock().unwrap();
                *running = false;
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

    log::info!("Service running");

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

    log::info!("Service stopping");

    Ok(())
}

fn touch(path: &Path) -> io::Result<()> {
    match OpenOptions::new().create(true).write(true).open(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

fn setup_local(service_name: &str) {
    let mut our_dir: String = Default::default();

    if let Some(base_dirs) = BaseDirs::new() {
        our_dir.push_str(base_dirs.data_local_dir().to_str().unwrap());
        our_dir.push_str("/");
        our_dir.push_str(service_name);
    }

    let mut log_file: String = Default::default();
    log_file.push_str(&our_dir);
    log_file.push_str("/");
    log_file.push_str("bot.log");

    println!("Path: {:?}", &Path::new(&log_file));

    touch(&Path::new(&log_file)).unwrap_or_else(|why| {
        println!("! {:?}", why.kind());
        log::error!("! {:?}", why.kind());
    });

    let config = LogConfigBuilder::builder()
        .path(log_file)
        .level("debug")
        .output_file()
        .output_console()
        .build();

    let _ = simple_log::new(config).unwrap();
}

async fn subscribe_to_pubsub(running_flag: Arc<Mutex<bool>>) {
    let cred = CredentialsFile::new_from_file("credentials.json".to_string()).await.unwrap();
    let client_config = ClientConfig::default().with_credentials(cred).await.unwrap();
    let client = Client::new(client_config).await.unwrap();
    let subscription = client.subscription("llm-prompt-sub");

    while *running_flag.lock().unwrap() {
        let message: Vec<ReceivedMessage> = subscription.pull(10, None).await.unwrap();
        for msg in message {
            print_message(&msg).await;
            msg.ack().await.unwrap();
        }
    }
}

async fn print_message(msg: &ReceivedMessage) {
    println!("Received message: {:?}", msg);
    log::info!("Received message: {:?}", msg);
}

fn main() -> windows_service::Result<()> {
    let service_name = "lm-bot";
    setup_local(service_name);
    service_dispatcher::start(service_name, ffi_service_main)?;
    Ok(())
}
