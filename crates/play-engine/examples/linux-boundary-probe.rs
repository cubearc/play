//! Linux-only adversarial checks; this binary is never packaged or exposed by IPC.
#[cfg(target_os = "linux")]
fn main() {
    use rustix::process::{Pid, Resource, Signal, getrlimit, kill_process};
    use std::{env, fs, io::ErrorKind, net::UdpSocket, os::unix::net::UnixStream, thread};

    let mut args: Vec<_> = env::args().skip(1).collect();
    let terminal = args.first().is_some_and(|arg| arg == "terminal");
    if terminal {
        args.remove(0);
    }
    let restrict = if terminal {
        play_engine::native::restrict_terminal
    } else {
        play_engine::native::restrict
    };
    if args == ["unavailable"]
        || args == ["seccomp-unavailable"]
        || args == ["thread-seal-unavailable"]
    {
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
        let thread_seal = args == ["thread-seal-unavailable"];
        let rules = if thread_seal {
            vec![
                seccompiler::SeccompRule::new(vec![
                    seccompiler::SeccompCondition::new(
                        1,
                        seccompiler::SeccompCmpArgLen::Qword,
                        seccompiler::SeccompCmpOp::Eq,
                        libc::SECCOMP_FILTER_FLAG_TSYNC as u64,
                    )
                    .unwrap(),
                ])
                .unwrap(),
            ]
        } else {
            vec![]
        };
        let filter: BpfProgram = SeccompFilter::new(
            std::collections::BTreeMap::from([(syscall, rules)]),
            SeccompAction::Allow,
            SeccompAction::Errno(libc::ENOSYS as u32),
            env::consts::ARCH.try_into().unwrap(),
        )
        .unwrap()
        .try_into()
        .unwrap();
        seccompiler::apply_filter(&filter).unwrap();
        if thread_seal {
            restrict().unwrap();
            assert_eq!(
                play_engine::native::seal_threads(),
                Err("thread_policy_failed")
            );
        } else {
            assert_eq!(restrict(), Err(expected));
        }
        println!("refused");
        return;
    }
    let [path, pid] = args.as_slice() else {
        panic!("explicit owned test inputs required")
    };
    let pid = Pid::from_raw(pid.parse().unwrap()).unwrap();
    let original = getrlimit(Resource::Nproc);
    restrict().unwrap();
    assert_eq!(getrlimit(Resource::Nproc), original);
    // Even the terminal policy cannot change device state or query the socket
    // as a terminal. Only inherited stdin/stderr read-only queries are granted.
    assert_eq!(
        rustix::termios::tcsetwinsize(
            std::io::stdin(),
            rustix::termios::Winsize {
                ws_col: 80,
                ws_row: 24,
                ws_xpixel: 0,
                ws_ypixel: 0,
            }
        ),
        Err(rustix::io::Errno::PERM)
    );
    assert_eq!(
        rustix::termios::tcgetwinsize(std::io::stdout()),
        Err(rustix::io::Errno::PERM)
    );
    assert_eq!(getrlimit(Resource::Nofile).maximum, Some(32));
    assert_eq!(getrlimit(Resource::Core).maximum, Some(0));
    assert_eq!(getrlimit(Resource::Cpu).maximum, Some(1));
    assert_eq!(getrlimit(Resource::Fsize).maximum, Some(16384));
    assert_eq!(
        kill_process(pid, Signal::KILL),
        Err(rustix::io::Errno::PERM)
    );
    let path = path.clone();
    let (release, ready) = std::sync::mpsc::sync_channel(0);
    let child = thread::spawn(move || {
        ready.recv().unwrap();
        no_more_threads();
        use std::os::unix::process::CommandExt;
        assert_eq!(
            std::process::Command::new("/usr/bin/true").exec().kind(),
            ErrorKind::PermissionDenied
        );
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
    });
    play_engine::native::seal_threads().unwrap();
    no_more_threads();
    // The initialized owner thread cannot install another filter either.
    assert_eq!(
        play_engine::native::seal_threads(),
        Err("thread_policy_failed")
    );
    release.send(()).unwrap();
    child.join().unwrap();
    println!("isolated");
}

#[cfg(target_os = "linux")]
fn no_more_threads() {
    match std::thread::Builder::new().spawn(|| ()) {
        Err(error) => assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied),
        Ok(thread) => {
            thread.join().unwrap();
            panic!("sealed thread set allowed creation");
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("Linux policy required");
    std::process::exit(1);
}
