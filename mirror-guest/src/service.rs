use std::env;
use std::ffi::OsString;
use std::{thread, time::Duration};

use log::{error, info, LevelFilter};

use windows_service::{
    service::{
        ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
        ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult, ServiceStatusHandle},
    service_manager::{ServiceManager, ServiceManagerAccess},
};

pub const SERVICE_NAME: &str = "mirror-guest";

pub fn is_installed() -> bool {
    if let Ok(service_manager) =
        ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
    {
        service_manager
            .open_service(SERVICE_NAME, ServiceAccess::QUERY_STATUS)
            .is_ok()
    } else {
        false
    }
}

pub fn install_service() -> windows_service::Result<()> {
    println!("trying to install mirror-guest service");

    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_binary_path = ::std::env::current_exe().unwrap();

    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from("memflow mirror guest agent"),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: service_binary_path,
        launch_arguments: vec!["service".into()],
        dependencies: vec![],
        account_name: None, // run as System
        account_password: None,
    };
    let service = service_manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;
    service.set_description("memflow mirror guest agent service")?;

    println!("mirror-guest service installed");
    Ok(())
}

pub fn uninstall_service() -> windows_service::Result<()> {
    println!("trying to uninstall mirror-guest service");

    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = service_manager.open_service(SERVICE_NAME, service_access)?;

    let service_status = service.query_status()?;
    if service_status.current_state != ServiceState::Stopped {
        service.stop()?;
        // Wait for service to stop
        thread::sleep(Duration::from_secs(1));
    }

    service.delete()?;

    println!("mirror-guest service uninstalled");
    Ok(())
}

pub fn get_handle() -> windows_service::Result<ServiceStatusHandle> {
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop | ServiceControl::Interrogate => {
                std::process::exit(0);
                //ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    // Tell the system that the service is running now
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    Ok(status_handle)
}
