use crate::domain::models::ProcessInfo;

use libc::{SCHED_BATCH, SCHED_FIFO, SCHED_IDLE, SCHED_OTHER, SCHED_RR, SYS_ioprio_get, SYS_sched_getattr, syscall};
use std::collections::HashSet;
use std::fs;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

pub struct ProcessService
{
    system: System,
    active_process_names: HashSet<String>,
}

impl ProcessService
{
    pub fn new() -> Self
    {
        let mut service = Self {
            system: System::new_with_specifics(RefreshKind::nothing().with_processes(ProcessRefreshKind::everything())),
            active_process_names: HashSet::new(),
        };

        service.update_processes();
        service
    }

    pub fn get_process_infos(&self, rule_name: &str) -> Vec<ProcessInfo>
    {
        let query = rule_name.to_lowercase();
        let mut process_infos = Vec::new();

        for (pid, process) in self.system.processes()
        {
            if process.name().to_string_lossy().to_lowercase() == query
            {
                let pid_int = pid.as_u32() as i32;

                let nice = self.read_nice(pid_int);
                let oom_score_adj = self.read_oom_score(pid_int);
                let cgroup = self.read_cgroup(pid_int);
                let (sched_policy, rtprio, latency_nice) = self.read_scheduler_info(pid_int);
                let ioclass = self.read_io_priority(pid_int);

                process_infos.push(ProcessInfo {
                    pid: pid_int,
                    name: process.name().to_string_lossy().to_string(),
                    nice,
                    oom_score_adj,
                    cgroup,
                    sched_policy,
                    rtprio,
                    ioclass,
                    latency_nice,
                });
            }
        }
        process_infos
    }

    pub fn is_process_active(&self, rule_name: &str) -> bool
    {
        self.active_process_names.contains(&rule_name.to_lowercase())
    }

    pub fn shorten_cgroup(path: &str) -> String
    {
        if path == "/"
        {
            return path.to_string();
        }

        let parts: Vec<&str> = path.split('/').collect();

        if parts.len() > 4 && path.starts_with("/user.slice")
        {
            let end = parts[parts.len().saturating_sub(2)..].join("/");

            return format!(".../{}", end);
        }

        path.to_string()
    }

    pub fn update_processes(&mut self)
    {
        self.system.refresh_processes(ProcessesToUpdate::All, true);
        self.active_process_names = self
            .system
            .processes()
            .values()
            .map(|p| p.name().to_string_lossy().to_lowercase())
            .collect();
    }

    fn read_nice(&self, pid: i32) -> Option<i32>
    {
        unsafe {
            let val = libc::getpriority(0, pid as u32);

            if val >= -20 && val <= 19 { Some(val) } else { Some(val) }
        }
    }

    fn read_oom_score(&self, pid: i32) -> Option<i32>
    {
        let path = format!("/proc/{}/oom_score_adj", pid);

        fs::read_to_string(path)
            .ok()
            .and_then(|content| content.trim().parse().ok())
    }

    fn read_cgroup(&self, pid: i32) -> Option<String>
    {
        let path = format!("/proc/{}/cgroup", pid);
        let content = fs::read_to_string(path).ok()?;

        for line in content.lines()
        {
            let parts: Vec<&str> = line.split(':').collect();

            if parts.len() == 3
            {
                let cgroup_path = parts[2];
                if cgroup_path != "/" && !cgroup_path.is_empty()
                {
                    return Some(cgroup_path.to_string());
                }
            }
        }

        Some("/".to_string())
    }

    fn read_scheduler_info(&self, pid: i32) -> (Option<String>, Option<i32>, Option<i32>)
    {
        unsafe {
            let policy_result = libc::sched_getscheduler(pid);
            let policy = if policy_result >= 0
            {
                match policy_result
                {
                    SCHED_OTHER => Some("normal".to_string()),
                    SCHED_FIFO => Some("fifo".to_string()),
                    SCHED_RR => Some("rr".to_string()),
                    SCHED_BATCH => Some("batch".to_string()),
                    SCHED_IDLE => Some("idle".to_string()),
                    6 => Some("deadline".to_string()),
                    _ => Some(format!("unknown({})", policy_result)),
                }
            }
            else
            {
                None
            };

            let mut param: libc::sched_param = std::mem::zeroed();
            let rtprio = if libc::sched_getparam(pid, &mut param) == 0
            {
                Some(param.sched_priority)
            }
            else
            {
                None
            };

            #[repr(C)]
            struct SchedAttr
            {
                size: u32,
                sched_policy: u32,
                sched_flags: u64,
                sched_nice: i32,
                sched_priority: u32,
                sched_runtime: u64,
                sched_deadline: u64,
                sched_period: u64,
                sched_util_min: u32,
                sched_util_max: u32,
                sched_latency_nice: i32,
            }

            let mut attr: SchedAttr = std::mem::zeroed();
            attr.size = std::mem::size_of::<SchedAttr>() as u32;

            let result = syscall(SYS_sched_getattr, pid, &mut attr as *mut SchedAttr, attr.size, 0);

            let latency_nice = if result == 0
            {
                Some(attr.sched_latency_nice)
            }
            else
            {
                None
            };

            (policy, rtprio, latency_nice)
        }
    }

    fn read_io_priority(&self, pid: i32) -> Option<String>
    {
        unsafe {
            let result = syscall(SYS_ioprio_get, 1, pid);
            if result >= 0
            {
                let value = result as i32;
                let ioclass_id = value >> 13;

                let ioclass = match ioclass_id
                {
                    0 => Some("none".to_string()),
                    1 => Some("realtime".to_string()),
                    2 => Some("best-effort".to_string()),
                    3 => Some("idle".to_string()),
                    _ => Some(format!("unknown({})", ioclass_id)),
                };

                ioclass
            }
            else
            {
                None
            }
        }
    }
}
