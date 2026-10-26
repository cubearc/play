use super::Config;
use landlock::{
    ABI, Access, AccessFs, CompatLevel, Compatible, PathBeneath, PathFd, Ruleset, RulesetAttr,
    RulesetCreatedAttr, RulesetStatus,
};
use nix::sys::resource::{Resource, setrlimit};
use std::{io, path::Path};

/// Called in the single-threaded console before threads or CLI children are created.
pub(super) fn apply(config: &Config, temporary: &Path) -> io::Result<()> {
    let fail = || {
        io::Error::other(
            "Runner filesystem isolation is unavailable. Use a Linux kernel with Landlock ABI 3 and an enabled Landlock LSM; no unrestricted terminal was started.",
        )
    };
    let read = AccessFs::ReadFile | AccessFs::ReadDir;
    let mut rules = Ruleset::default()
        .set_compatibility(CompatLevel::HardRequirement)
        .handle_access(AccessFs::from_all(ABI::V3))
        .map_err(|error| {
            io::Error::other(format!("Required filesystem rights unavailable: {error}"))
        })?
        .create()
        .map_err(|error| io::Error::other(format!("Landlock ruleset unavailable: {error}")))?;
    for path in [&config.source, &config.imports] {
        rules = rules
            .add_rule(PathBeneath::new(
                PathFd::new(path).map_err(|_| fail())?,
                read,
            ))
            .map_err(|_| fail())?;
    }
    for path in [config.output.as_path(), temporary] {
        let write = read
            | AccessFs::WriteFile
            | AccessFs::Truncate
            | AccessFs::MakeReg
            | AccessFs::MakeDir
            | AccessFs::RemoveFile
            | AccessFs::RemoveDir
            | AccessFs::Refer;
        rules = rules
            .add_rule(PathBeneath::new(
                PathFd::new(path).map_err(|_| fail())?,
                write,
            ))
            .map_err(|_| fail())?;
    }
    for path in ["/usr/lib", "/lib", "/lib64"] {
        if Path::new(path).exists() {
            rules = rules
                .add_rule(PathBeneath::new(
                    PathFd::new(path).map_err(|_| fail())?,
                    read | AccessFs::Execute,
                ))
                .map_err(|_| fail())?;
        }
    }
    rules = rules
        .add_rule(PathBeneath::new(
            PathFd::new(&config.cube).map_err(|_| fail())?,
            AccessFs::ReadFile | AccessFs::Execute,
        ))
        .map_err(|_| fail())?;
    for path in ["/etc/ld.so.cache", "/dev/null", "/dev/tty"] {
        if Path::new(path).exists() {
            let access = if path.starts_with("/dev/") {
                AccessFs::ReadFile | AccessFs::WriteFile
            } else {
                AccessFs::ReadFile.into()
            };
            rules = rules
                .add_rule(PathBeneath::new(
                    PathFd::new(path).map_err(|_| fail())?,
                    access,
                ))
                .map_err(|_| fail())?;
        }
    }
    let status = rules.restrict_self().map_err(|_| fail())?;
    if status.ruleset != RulesetStatus::FullyEnforced || !status.no_new_privs {
        return Err(fail());
    }
    for (resource, limit) in [
        (Resource::RLIMIT_CORE, 0),
        (Resource::RLIMIT_NOFILE, 64),
        (Resource::RLIMIT_NPROC, 128),
        (Resource::RLIMIT_AS, 256 * 1024 * 1024),
        (Resource::RLIMIT_FSIZE, 32 * 1024 * 1024),
        (Resource::RLIMIT_CPU, 20),
    ] {
        setrlimit(resource, limit, limit)
            .map_err(|_| io::Error::other("Runner process limits could not be enforced"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nix::sys::resource::getrlimit;
    use std::process::Command;

    #[test]
    fn filesystem_and_resource_limits_are_enforced_in_a_child() {
        const CHILD: &str = "CUBE_RUNNER_SANDBOX_TEST_CHILD";
        if std::env::var_os(CHILD).is_none() {
            use std::os::unix::{fs::PermissionsExt, process::CommandExt};
            let fixture = tempfile::tempdir().unwrap();
            let probe = fixture.path().join("resource-probe");
            assert!(
                Command::new("rustc")
                    .args(["--edition=2024", "--forbid", "unsafe_code"])
                    .arg(
                        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                            .join("tests/support/resource_probe.rs")
                    )
                    .arg("-o")
                    .arg(&probe)
                    .status()
                    .unwrap()
                    .success()
            );
            let mut child = Command::new(std::env::current_exe().unwrap());
            child.args([
                "--exact",
                "runner::sandbox::tests::filesystem_and_resource_limits_are_enforced_in_a_child",
                "--nocapture",
            ]).env(CHILD, fixture.path().join("child")).env("CUBE_RUNNER_RESOURCE_PROBE", &probe);
            if nix::unistd::geteuid().is_root() {
                // Only this empty, random fixture parent is shared with the test account.
                std::fs::set_permissions(fixture.path(), std::fs::Permissions::from_mode(0o777))
                    .unwrap();
                child.uid(65532).gid(65532);
            }
            assert!(child.status().unwrap().success());
            return;
        }
        // Apply an irreversible sandbox only in the disposable test process.
        let root = std::path::PathBuf::from(std::env::var_os(CHILD).unwrap());
        std::fs::create_dir(&root).unwrap();
        let config = Config {
            socket: root.join("unused.sock"),
            source: root.join("source"),
            imports: root.join("imports"),
            output: root.join("output"),
            cube: std::env::var_os("CUBE_RUNNER_RESOURCE_PROBE")
                .unwrap()
                .into(),
        };
        let temporary = root.join("temporary");
        for path in [&config.source, &config.imports, &config.output, &temporary] {
            std::fs::create_dir(path).unwrap();
        }
        let source = config.source.join("original");
        let private = root.join("private.db");
        std::fs::write(&source, b"unchanged").unwrap();
        std::fs::write(&private, b"private").unwrap();
        std::os::unix::fs::symlink(&private, config.source.join("escape")).unwrap();
        apply(&config, &temporary).unwrap();
        assert_eq!(std::fs::read(&source).unwrap(), b"unchanged");
        for path in [&private, &config.source.join("escape")] {
            assert_eq!(
                std::fs::read(path).unwrap_err().kind(),
                io::ErrorKind::PermissionDenied
            );
        }
        assert_eq!(
            std::fs::write(&source, b"replaced").unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        std::fs::write(config.output.join("derived"), b"local").unwrap();
        std::fs::write(temporary.join("scratch"), b"temporary").unwrap();
        assert_eq!(
            getrlimit(Resource::RLIMIT_AS).unwrap(),
            (256 * 1024 * 1024, 256 * 1024 * 1024)
        );
        assert_eq!(getrlimit(Resource::RLIMIT_CPU).unwrap(), (20, 20));
        assert_eq!(getrlimit(Resource::RLIMIT_NPROC).unwrap(), (128, 128));
        assert_eq!(
            getrlimit(Resource::RLIMIT_FSIZE).unwrap(),
            (32 * 1024 * 1024, 32 * 1024 * 1024)
        );
        assert_eq!(getrlimit(Resource::RLIMIT_CORE).unwrap(), (0, 0));
        assert!(
            Vec::<u8>::new()
                .try_reserve_exact(512 * 1024 * 1024)
                .is_err()
        );
        // Verify production values above, then lower soft limits for bounded stress tests.
        use std::os::unix::process::ExitStatusExt;
        setrlimit(Resource::RLIMIT_CPU, 1, 20).unwrap();
        let status = Command::new(&config.cube).arg("cpu").status().unwrap();
        assert_eq!(status.signal(), Some(nix::libc::SIGXCPU));
        setrlimit(Resource::RLIMIT_CPU, 20, 20).unwrap();
        setrlimit(Resource::RLIMIT_NPROC, 32, 128).unwrap();
        assert!(
            Command::new(&config.cube)
                .arg("processes")
                .status()
                .unwrap()
                .success()
        );
        setrlimit(Resource::RLIMIT_NPROC, 128, 128).unwrap();
        setrlimit(Resource::RLIMIT_FSIZE, 1024 * 1024, 32 * 1024 * 1024).unwrap();
        let status = Command::new(&config.cube)
            .arg("file")
            .arg(config.output.join("bounded-file"))
            .status()
            .unwrap();
        assert_eq!(status.signal(), Some(nix::libc::SIGXFSZ));
        setrlimit(Resource::RLIMIT_FSIZE, 32 * 1024 * 1024, 32 * 1024 * 1024).unwrap();
        let mut descriptors = Vec::new();
        loop {
            match std::fs::File::open("/dev/null") {
                Ok(file) => descriptors.push(file),
                Err(error) => {
                    assert_eq!(error.raw_os_error(), Some(nix::libc::EMFILE));
                    break;
                }
            }
            assert!(descriptors.len() < 64);
        }
        // The parent removes fixtures; this child cannot remove the read-only source.
    }
}
