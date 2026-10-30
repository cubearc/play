//! Default-deny syscall boundary, including filesystem metadata and all sockets.
use crate::Result;
use seccompiler::{
    BpfProgram, SeccompAction as Action, SeccompCmpArgLen as Length, SeccompCmpOp as Op,
    SeccompCondition as Condition, SeccompFilter, SeccompRule as Rule,
};
use std::collections::BTreeMap;

fn condition(index: u8, op: Op, value: u64) -> Result<Condition> {
    Condition::new(index, Length::Qword, op, value).map_err(|_| "syscall_policy_failed")
}
fn rule(conditions: Vec<Condition>) -> Result<Vec<Rule>> {
    Ok(vec![
        Rule::new(conditions).map_err(|_| "syscall_policy_failed")?,
    ])
}
fn compile(
    rules: BTreeMap<i64, Vec<Rule>>,
    default: Action,
    matched: Action,
) -> Result<BpfProgram> {
    SeccompFilter::new(
        rules,
        default,
        matched,
        std::env::consts::ARCH
            .try_into()
            .map_err(|_| "native_policy_unavailable")?,
    )
    .map_err(|_| "syscall_policy_failed")?
    .try_into()
    .map_err(|_| "syscall_policy_failed")
}

pub(super) fn filters(terminal: bool) -> Result<[BpfProgram; 2]> {
    use libc::*;
    // glibc falls back to argument-inspectable clone only for ENOSYS. Stack
    // this denial before the allowlist; clone3's pointed-to flags stay forbidden.
    let no_clone3 = compile(
        BTreeMap::from([(SYS_clone3, vec![])]),
        Action::Allow,
        Action::Errno(ENOSYS as u32),
    )?;
    let mut allowed: BTreeMap<_, _> = [
        SYS_read,
        SYS_write,
        SYS_readv,
        SYS_writev,
        SYS_close,
        SYS_fstat,
        SYS_lseek,
        SYS_mmap,
        SYS_mprotect,
        SYS_munmap,
        SYS_mremap,
        SYS_madvise,
        SYS_brk,
        SYS_futex,
        SYS_clock_gettime,
        SYS_clock_getres,
        SYS_clock_nanosleep,
        SYS_nanosleep,
        SYS_sched_yield,
        SYS_sched_getaffinity,
        SYS_rt_sigaction,
        SYS_rt_sigprocmask,
        SYS_rt_sigreturn,
        SYS_sigaltstack,
        SYS_getpid,
        SYS_gettid,
        SYS_set_tid_address,
        SYS_set_robust_list,
        SYS_rseq,
        SYS_getrandom,
        SYS_exit,
        SYS_exit_group,
        SYS_clone3,
    ]
    .into_iter()
    .map(|number| (number, vec![]))
    .collect();
    let required = (CLONE_VM | CLONE_SIGHAND | CLONE_THREAD) as u64;
    let thread_flags = (CLONE_VM
        | CLONE_FS
        | CLONE_FILES
        | CLONE_SIGHAND
        | CLONE_THREAD
        | CLONE_SYSVSEM
        | CLONE_SETTLS
        | CLONE_PARENT_SETTID
        | CLONE_CHILD_SETTID
        | CLONE_CHILD_CLEARTID) as u64;
    allowed.insert(
        SYS_clone,
        rule(vec![
            condition(0, Op::MaskedEq(required), required)?,
            condition(0, Op::MaskedEq(!thread_flags), 0)?,
        ])?,
    );
    // Only current-process queries; no raised limits or signalling other owners.
    allowed.insert(
        SYS_prlimit64,
        rule(vec![condition(0, Op::Eq, 0)?, condition(2, Op::Eq, 0)?])?,
    );
    allowed.insert(
        SYS_tgkill,
        rule(vec![condition(0, Op::Eq, std::process::id() as u64)?])?,
    );
    allowed.insert(
        SYS_prctl,
        rule(vec![condition(0, Op::Eq, PR_SET_NAME as u64)?])?,
    );
    if terminal {
        terminal_channels(&mut allowed)?;
    }
    let bounded = compile(allowed, Action::Errno(EPERM as u32), Action::Allow)?;
    Ok([no_clone3, bounded])
}

// Only the pre-opened PTY and private socket are used by Cube's CLI adapter.
// No socket creation, path lookup, exec or destination-bearing send is allowed.
fn terminal_channels(allowed: &mut BTreeMap<i64, Vec<Rule>>) -> Result<()> {
    use libc::*;
    allowed.insert(SYS_dup, rule(vec![condition(0, Op::Eq, 1)?])?);
    allowed.insert(
        SYS_fcntl,
        rule(vec![condition(1, Op::Eq, F_DUPFD_CLOEXEC as u64)?])?,
    );
    allowed.insert(SYS_getpeername, vec![]);
    allowed.insert(SYS_recvfrom, rule(vec![condition(4, Op::Eq, 0)?])?);
    allowed.insert(SYS_sendto, rule(vec![condition(4, Op::Eq, 0)?])?);
    let mut queries = Vec::new();
    for fd in [0, 2] {
        for request in [TCGETS, TIOCGWINSZ] {
            queries.extend(rule(vec![
                condition(0, Op::Eq, fd)?,
                condition(1, Op::Eq, u64::from(request as u32))?,
            ])?);
        }
    }
    allowed.insert(SYS_ioctl, queries);
    Ok(())
}
