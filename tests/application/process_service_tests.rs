use ananicy_rule_o_matic::application::process_service::ProcessService;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn spawn_test_process(name: &str) -> std::process::Child
{
    Command::new(name)
        .arg("5")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn process")
}

#[test]
fn test_integration_process_detection()
{
    let mut child = spawn_test_process("sleep");
    let pid = child.id() as i32;

    let mut process_service = ProcessService::new();

    thread::sleep(Duration::from_millis(100));

    process_service.update_processes();

    let infos = process_service.get_process_infos("sleep");
    let found_pid = infos.iter().find(|info| info.pid == pid);

    let _ = child.kill();

    assert!(
        found_pid.is_some(),
        "Failed to detect spawned sleep process with PID {}",
        pid
    );

    assert_eq!(found_pid.unwrap().name, "sleep");
}

#[test]
fn test_is_process_active_found()
{
    let mut child = spawn_test_process("sleep");
    let mut process_service = ProcessService::new();

    thread::sleep(Duration::from_millis(100));
    process_service.update_processes();

    let is_active = process_service.is_process_active("sleep");

    let _ = child.kill();

    assert!(is_active, "Should return true for running process");
}

#[test]
fn test_is_process_active_not_found()
{
    let mut process_service = ProcessService::new();
    process_service.update_processes();

    let is_active = process_service.is_process_active("non_existent_process_12345");

    assert!(!is_active, "Should return false for non-existent process");
}

#[test]
fn test_process_info_fields()
{
    let mut child = spawn_test_process("sleep");
    let pid = child.id() as i32;

    let mut process_service = ProcessService::new();
    thread::sleep(Duration::from_millis(100));
    process_service.update_processes();

    let infos = process_service.get_process_infos("sleep");
    let info = infos.iter().find(|i| i.pid == pid).expect("Process not found");

    assert!(info.nice.is_some(), "Nice value should be present");

    if let Some(policy) = &info.sched_policy
    {
        assert!(
            ["normal", "fifo", "rr", "batch", "idle", "deadline"].contains(&policy.as_str())
                || policy.starts_with("unknown"),
            "Unknown scheduler policy: {}",
            policy
        );
    }

    if let Some(ioclass) = &info.ioclass
    {
        assert!(
            ["none", "realtime", "best-effort", "idle"].contains(&ioclass.as_str()) || ioclass.starts_with("unknown"),
            "Unknown io class: {}",
            ioclass
        );
    }

    let _ = child.kill();
}

#[test]
fn test_shorten_cgroup()
{
    assert_eq!(ProcessService::shorten_cgroup("/"), "/");

    assert_eq!(
        ProcessService::shorten_cgroup("/user.slice/user-1000.slice"),
        "/user.slice/user-1000.slice"
    );

    let long_path = "/user.slice/user-1000.slice/user@1000.service/app.slice/app-gnome-terminal.scope";
    let shortened = ProcessService::shorten_cgroup(long_path);

    assert_eq!(shortened, ".../app.slice/app-gnome-terminal.scope");

    let system_path = "/system.slice/system-dbus.slice/some.service/deeply/nested/cgroup";

    assert_eq!(ProcessService::shorten_cgroup(system_path), system_path);
}
