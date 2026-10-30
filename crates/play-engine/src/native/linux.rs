//! Pure workers have no path grants: all inputs arrive on inherited private pipes.
mod syscalls;

use crate::Result;
use landlock::{
    ABI, Access, AccessFs, CompatLevel, Compatible, Ruleset, RulesetAttr, RulesetStatus,
};
use rustix::process::{Resource, Rlimit, setrlimit};

pub(super) fn restrict() -> Result<()> {
    apply(syscalls::filters(false)?)
}

pub(super) fn restrict_terminal() -> Result<()> {
    apply(syscalls::filters(true)?)
}

fn apply(filters: [seccompiler::BpfProgram; 2]) -> Result<()> {
    // Prepare filters and measure runtime mappings while still single-threaded.
    let memory = memory_limit()?;
    limits(memory)?;
    let status = Ruleset::default()
        .set_compatibility(CompatLevel::HardRequirement)
        .handle_access(AccessFs::from_all(ABI::V3))
        .map_err(|_| "filesystem_policy_failed")?
        .create()
        .map_err(|_| "filesystem_policy_failed")?
        .restrict_self()
        .map_err(|_| "filesystem_policy_failed")?;
    if status.ruleset != RulesetStatus::FullyEnforced || !status.no_new_privs {
        return Err("filesystem_policy_failed");
    }
    for filter in filters {
        seccompiler::apply_filter(&filter).map_err(|_| "syscall_policy_failed")?;
    }
    Ok(())
}

fn memory_limit() -> Result<u64> {
    let status =
        std::fs::read_to_string("/proc/self/status").map_err(|_| "memory_policy_failed")?;
    let initial = status.lines().find_map(|line| {
        let mut fields = line.strip_prefix("VmSize:")?.split_whitespace();
        let value = fields.next()?.parse::<u64>().ok()?;
        (fields.next()? == "kB").then_some(value)?.checked_mul(1024)
    });
    initial
        .and_then(|bytes| bytes.checked_add(crate::protocol::contract().vm_growth_bytes))
        .ok_or("memory_policy_failed")
}

fn limits(memory: u64) -> Result<()> {
    let contract = crate::protocol::contract();
    // RLIMIT_NPROC counts the host UID, not this worker. Process creation is
    // denied by seccomp; the existing scheduler bounds workers and node threads.
    for (resource, limit, code) in [
        (Resource::Core, 0, "core_policy_failed"),
        (Resource::Cpu, contract.cpu_seconds, "cpu_policy_failed"),
        (Resource::As, memory, "memory_policy_failed"),
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
