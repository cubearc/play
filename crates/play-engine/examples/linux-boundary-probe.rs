//! Linux-only adversarial checks; this binary is never packaged or exposed by IPC.
#[cfg(target_os = "linux")]
fn main() {
    use rustix::process::{Pid, Resource, Signal, getrlimit, kill_process};
    use std::{env, fs, io::ErrorKind, net::UdpSocket, os::unix::net::UnixStream, thread};

    let args: Vec<_> = env::args().skip(1).collect();
    if args == ["unavailable"] || args == ["seccomp-unavailable"] {
        // Simulate a host refusing a mandatory policy syscall, without a
        // production bypass or fallback. The worker must refuse its entire policy.
        use seccompiler::{BpfProgram, SeccompAction, SeccompFilter};
        let (syscall, expected) = if args == ["unavailable"] {
            (
                libc::SYS_landlock_create_ruleset,
                "filesystem_policy_failed",
            )
        } else {
            (libc::SYS_seccomp, "syscall_policy_failed")
        };
        let filter: BpfProgram = SeccompFilter::new(
            std::collections::BTreeMap::from([(syscall, vec![])]),
            SeccompAction::Allow,
            SeccompAction::Errno(libc::ENOSYS as u32),
            env::consts::ARCH.try_into().unwrap(),
        )
        .unwrap()
        .try_into()
        .unwrap();
        seccompiler::apply_filter(&filter).unwrap();
        assert_eq!(play_engine::native::restrict(), Err(expected));
        println!("refused");
        return;
    }
    let [path, pid] = args.as_slice() else {
        panic!("explicit owned test inputs required")
    };
    let pid = Pid::from_raw(pid.parse().unwrap()).unwrap();
    let original = getrlimit(Resource::Nproc);
    play_engine::native::restrict().unwrap();
    assert_eq!(getrlimit(Resource::Nproc), original);
    assert_eq!(getrlimit(Resource::Nofile).maximum, Some(32));
    assert_eq!(getrlimit(Resource::Core).maximum, Some(0));
    assert_eq!(getrlimit(Resource::Cpu).maximum, Some(1));
    assert_eq!(getrlimit(Resource::Fsize).maximum, Some(16384));
    assert_eq!(
        kill_process(pid, Signal::KILL),
        Err(rustix::io::Errno::PERM)
    );
    let path = path.clone();
    thread::spawn(move || {
        assert!(fs::metadata(&path).is_err_and(|e| e.kind() == ErrorKind::PermissionDenied));
        assert!(fs::read(&path).is_err_and(|e| e.kind() == ErrorKind::PermissionDenied));
        assert!(
            fs::write(&path, b"forbidden").is_err_and(|e| e.kind() == ErrorKind::PermissionDenied)
        );
        assert!(
            UdpSocket::bind("127.0.0.1:0").is_err_and(|e| e.kind() == ErrorKind::PermissionDenied)
        );
        assert!(UnixStream::pair().is_err_and(|e| e.kind() == ErrorKind::PermissionDenied));
        assert!(
            std::process::Command::new("/usr/bin/true")
                .status()
                .is_err_and(|e| e.kind() == ErrorKind::PermissionDenied)
        );
    })
    .join()
    .unwrap();
    println!("isolated");
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("Linux policy required");
    std::process::exit(1);
}
