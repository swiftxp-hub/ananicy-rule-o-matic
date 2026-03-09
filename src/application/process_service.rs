use crate::domain::models::ProcessInfo;

use libc::{SCHED_BATCH, SCHED_FIFO, SCHED_IDLE, SCHED_OTHER, SCHED_RR, SYS_ioprio_get, SYS_sched_getattr, syscall};
use std::borrow::Cow;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

pub struct ProcessService
{
    system: System,
    process_names: HashSet<String>,
    truncated_process_names: HashSet<String>,
}

impl ProcessService
{
    pub fn new() -> Self
    {
        let mut process_service = Self {
            system: System::new_with_specifics(RefreshKind::nothing().with_processes(ProcessRefreshKind::everything())),
            process_names: HashSet::new(),
            truncated_process_names: HashSet::new(),
        };

        process_service.update_processes();
        process_service
    }

    pub fn get_process_infos(&self, rule_name: &str) -> Vec<ProcessInfo>
    {
        let mut process_infos = Vec::new();

        for (pid, process) in self.system.processes()
        {
            let process_name = process.name().to_string_lossy();

            if self.matches_rule_name(rule_name, process)
            {
                let pid_int = pid.as_u32() as i32;

                let nice = self.read_nice(pid_int);
                let oom_score_adj = self.read_oom_score(pid_int);
                let cgroup = self.read_cgroup(pid_int);
                let (sched_policy, rtprio, latency_nice) = self.read_scheduler_info(pid_int);
                let ioclass = self.read_io_priority(pid_int);

                process_infos.push(ProcessInfo {
                    process_id: pid_int,
                    name: process_name.to_string(),
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
        let rule_lower = rule_name.to_lowercase();

        if self.process_names.contains(&rule_lower)
        {
            return true;
        }

        if let Some(stripped) = rule_lower.strip_suffix(".exe")
        {
            if self.process_names.contains(stripped)
            {
                return true;
            }
        }

        let with_exe = format!("{}.exe", rule_lower);
        if self.process_names.contains(&with_exe)
        {
            return true;
        }

        if rule_lower.len() > 15
        {
            let truncated = &rule_lower[..15];
            if self.truncated_process_names.contains(truncated)
            {
                return true;
            }
        }

        false
    }

    fn matches_rule_name(&self, rule_name: &str, process: &sysinfo::Process) -> bool
    {
        let proc_name = process.name().to_string_lossy();
        if self.check_name_match(rule_name, &proc_name)
        {
            return true;
        }

        for arg in process.cmd()
        {
            let path = Path::new(arg);
            if let Some(file_name) = path.file_name()
            {
                let name = file_name.to_string_lossy();
                if self.check_name_match(rule_name, &name)
                {
                    return true;
                }
            }
        }

        false
    }

    fn check_name_match(&self, rule_name: &str, proc_name: &str) -> bool
    {
        if rule_name.eq_ignore_ascii_case(proc_name)
        {
            return true;
        }

        if proc_name.len() > 4
            && proc_name[proc_name.len() - 4..].eq_ignore_ascii_case(".exe")
            && proc_name[..proc_name.len() - 4].eq_ignore_ascii_case(rule_name)
        {
            return true;
        }

        if rule_name.len() > 4
            && rule_name[rule_name.len() - 4..].eq_ignore_ascii_case(".exe")
            && rule_name[..rule_name.len() - 4].eq_ignore_ascii_case(proc_name)
        {
            return true;
        }

        if proc_name.len() == 15 && rule_name.len() > 15 && rule_name[..15].eq_ignore_ascii_case(proc_name)
        {
            return true;
        }

        false
    }

    pub fn shorten_cgroup(cgroup_path: &str) -> Cow<'_, str>
    {
        if cgroup_path == "/"
        {
            return Cow::Borrowed(cgroup_path);
        }

        let cgroup_parts: Vec<&str> = cgroup_path.split('/').collect();

        if cgroup_parts.len() > 4 && cgroup_path.starts_with("/user.slice")
        {
            let cgroup_end = cgroup_parts[cgroup_parts.len().saturating_sub(2)..].join("/");

            return Cow::Owned(format!(".../{}", cgroup_end));
        }

        Cow::Borrowed(cgroup_path)
    }

    pub fn search_processes(&self, query: &str) -> Vec<String>
    {
        let query_lower = query.to_lowercase();
        let mut results: Vec<String> = self
            .process_names
            .iter()
            .filter(|name| name.contains(&query_lower))
            .cloned()
            .collect();
        results.sort();
        results
    }

    pub fn update_processes(&mut self)
    {
        self.system.refresh_processes(ProcessesToUpdate::All, true);

        self.process_names.clear();
        self.truncated_process_names.clear();

        for process in self.system.processes().values()
        {
            let name = process.name().to_string_lossy().to_lowercase();
            if name.len() == 15
            {
                self.truncated_process_names.insert(name.clone());
            }
            self.process_names.insert(name);

            for arg in process.cmd()
            {
                let path = Path::new(arg);
                if let Some(file_name) = path.file_name()
                {
                    let name = file_name.to_string_lossy().to_lowercase();
                    self.process_names.insert(name);
                }
            }
        }
    }

    fn read_nice(&self, pid: i32) -> Option<i32>
    {
        unsafe {
            let val = libc::getpriority(0, pid as u32);

            Some(val)
        }
    }

    fn read_oom_score(&self, pid: i32) -> Option<i32>
    {
        let oom_score_path = format!("/proc/{}/oom_score_adj", pid);

        fs::read_to_string(oom_score_path)
            .ok()
            .and_then(|content| content.trim().parse().ok())
    }

    fn read_cgroup(&self, pid: i32) -> Option<String>
    {
        let cgroup_path = format!("/proc/{}/cgroup", pid);
        let cgroup_content = fs::read_to_string(cgroup_path).ok()?;

        for cgroup_line in cgroup_content.lines()
        {
            let cgroup_line_parts: Vec<&str> = cgroup_line.split(':').collect();

            if cgroup_line_parts.len() == 3
            {
                let cgroup_path = cgroup_line_parts[2];
                if cgroup_path != "/" && !cgroup_path.is_empty()
                {
                    return Some(cgroup_path.to_string());
                }
            }
        }

        Some("/".to_string())
    }

    fn read_scheduler_info(&self, process_id: i32) -> (Option<String>, Option<i32>, Option<i32>)
    {
        unsafe {
            let policy_result = libc::sched_getscheduler(process_id);
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

            let mut sched_priority_param: libc::sched_param = std::mem::zeroed();
            let rtprio = if libc::sched_getparam(process_id, &mut sched_priority_param) == 0
            {
                Some(sched_priority_param.sched_priority)
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

            let mut latency_nice_attribute: SchedAttr = std::mem::zeroed();
            latency_nice_attribute.size = std::mem::size_of::<SchedAttr>() as u32;

            let result = syscall(
                SYS_sched_getattr,
                process_id,
                &mut latency_nice_attribute as *mut SchedAttr,
                latency_nice_attribute.size,
                0,
            );

            let latency_nice = if result == 0
            {
                Some(latency_nice_attribute.sched_latency_nice)
            }
            else
            {
                None
            };

            (policy, rtprio, latency_nice)
        }
    }

    fn read_io_priority(&self, process_id: i32) -> Option<String>
    {
        unsafe {
            let io_priority = syscall(SYS_ioprio_get, 1, process_id);

            if io_priority >= 0
            {
                let io_priority_value = io_priority as i32;
                let ioclass_id = io_priority_value >> 13;

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
