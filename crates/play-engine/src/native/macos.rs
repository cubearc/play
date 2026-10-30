use crate::Result;

pub(super) fn restrict() -> Result<()> {
    use rustix::process::{Resource, Rlimit, setrlimit};
    let contract = crate::protocol::contract();
    // Darwin counts the already mapped runtime/shared cache in this limit.
    // Bound additional VM; do not describe this as an RSS/heap ceiling.
    let initial =
        libproc::proc_pid::pidinfo::<libproc::task_info::TaskInfo>(std::process::id() as i32, 0)
            .map_err(|_| "memory_policy_failed")?
            .pti_virtual_size;
    let vm_limit = initial
        .checked_add(contract.vm_growth_bytes)
        .ok_or("memory_policy_failed")?;
    for (resource, limit, code) in [
        (Resource::Core, 0, "core_policy_failed"),
        (Resource::Cpu, contract.cpu_seconds, "cpu_policy_failed"),
        (Resource::Data, vm_limit, "memory_policy_failed"),
        (Resource::Nproc, 0, "process_policy_failed"),
        (Resource::Nofile, 32, "descriptor_policy_failed"),
        (
            Resource::Fsize,
            contract.output_bytes as u64,
            "file_policy_failed",
        ),
    ] {
        setrlimit(
            resource,
            Rlimit {
                current: Some(limit),
                maximum: Some(limit),
            },
        )
        .map_err(|_| code)?;
    }
    Ok(())
}
