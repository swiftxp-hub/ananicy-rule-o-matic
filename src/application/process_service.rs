use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

pub struct ProcessService
{
    system: System,
}

impl ProcessService
{
    pub fn new() -> Self
    {
        Self {
            system: System::new_with_specifics(RefreshKind::nothing().with_processes(ProcessRefreshKind::everything())),
        }
    }

    pub fn update_processes(&mut self)
    {
        self.system.refresh_processes(ProcessesToUpdate::All, true);
    }

    pub fn is_process_active(&self, rule_name: &str) -> bool
    {
        let processes = self.system.processes();
        let query = rule_name.to_lowercase();

        processes
            .values()
            .any(|process| process.name().to_string_lossy().to_lowercase() == query)
    }
}
