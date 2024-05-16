mod src;
pub use src::app_config::lm_bot;
use std::{path::PathBuf, env};

#[cfg(windows)]
// fn main() {
//     let dir = PathBuf::from(env::var("CRATE_OUT_DIR").unwrap());
//     let exe = dir.join(lm_bot::EXE_FILENAME);
//     println!("{:?}", exe);
// }
fn main() -> windows_service::Result<()> {
    use std::ffi::OsString;
    use windows_service::{
        service::{ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType},
        service_manager::{ServiceManager, ServiceManagerAccess},
    };

    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    // This example installs the service defined in `examples/ping_service.rs`.
    // In the real world code you would set the executable path to point to your own binary
    // that implements windows service.
    let dir = PathBuf::from(env::var("CRATE_OUT_DIR").unwrap());
    let exe = dir.join(lm_bot::EXE_FILENAME);

    let service_info = ServiceInfo {
        name: OsString::from(lm_bot::SERVICE_NAME),
        display_name: OsString::from(lm_bot::DISPLAY_NAME),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::OnDemand,
        error_control: ServiceErrorControl::Normal,
        executable_path: exe,
        launch_arguments: vec![],
        dependencies: vec![],
        account_name: None, // run as System
        account_password: None,
    };

    let service = service_manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;
    service.set_description(lm_bot::SERVICE_DESCRIPTION)?;
    
    Ok(())
}