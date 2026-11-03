use super::{
    handles::{checked, owned, raw, wide},
    profile::Profile,
};
use std::{
    ffi::OsStr,
    fs::File,
    io,
    mem::{size_of, zeroed},
    os::windows::{io::OwnedHandle, process::ExitStatusExt},
    path::Path,
    process::ExitStatus,
    ptr,
};
use tempfile::TempDir;
use windows_sys::Win32::{
    Foundation::*,
    Security::{SECURITY_ATTRIBUTES, SECURITY_CAPABILITIES},
    System::{JobObjects::*, Pipes::CreatePipe, Threading::*},
};

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub private_memory_bytes: usize,
    pub cpu_seconds: u64,
}
impl Limits {
    pub(crate) fn validate(self) -> io::Result<()> {
        if !(16 * 1024 * 1024..=256 * 1024 * 1024).contains(&self.private_memory_bytes)
            || !(1..=10).contains(&self.cpu_seconds)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid native limits",
            ));
        }
        Ok(())
    }
}

struct Attributes {
    buffer: Vec<usize>,
    initialized: bool,
}
impl Attributes {
    fn new() -> io::Result<Self> {
        let mut bytes = 0;
        // SAFETY: the first call asks Win32 for the exact buffer size.
        unsafe {
            InitializeProcThreadAttributeList(ptr::null_mut(), 2, 0, &mut bytes);
        }
        if bytes == 0 || bytes > 65536 {
            return Err(io::Error::other("invalid attribute size"));
        }
        let mut result = Self {
            buffer: vec![0; bytes.div_ceil(size_of::<usize>())],
            initialized: false,
        };
        // SAFETY: the aligned buffer has the requested size and remains owned by Self.
        checked(unsafe { InitializeProcThreadAttributeList(result.pointer(), 2, 0, &mut bytes) })?;
        result.initialized = true;
        Ok(result)
    }
    fn pointer(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.buffer.as_mut_ptr().cast()
    }
    fn set<T>(&mut self, key: usize, value: &mut T) -> io::Result<()> {
        // SAFETY: each supplied attribute remains live through CreateProcessW.
        checked(unsafe {
            UpdateProcThreadAttribute(
                self.pointer(),
                0,
                key,
                (value as *mut T).cast(),
                size_of::<T>(),
                ptr::null_mut(),
                ptr::null(),
            )
        })
    }
}
impl Drop for Attributes {
    fn drop(&mut self) {
        // SAFETY: only initialized attribute lists reach this owning destructor.
        if self.initialized {
            // SAFETY: only a successfully initialized list is deleted.
            unsafe {
                DeleteProcThreadAttributeList(self.pointer());
            }
        }
    }
}
fn pipe() -> io::Result<(OwnedHandle, OwnedHandle)> {
    let (mut read, mut write) = (ptr::null_mut(), ptr::null_mut());
    let sa = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: ptr::null_mut(),
        bInheritHandle: 1,
    };
    // SAFETY: valid output pointers and explicit inheritable pipe attributes.
    checked(unsafe { CreatePipe(&mut read, &mut write, &sa, 0) })?;
    Ok((owned(read)?, owned(write)?))
}
fn job(limits: Limits) -> io::Result<OwnedHandle> {
    limits.validate()?;
    // SAFETY: an unnamed, non-inheritable job owned solely by the trusted parent.
    let job = owned(unsafe { CreateJobObjectW(ptr::null(), ptr::null()) })?;
    // SAFETY: the Win32 structure is valid when zero-initialized.
    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
        | JOB_OBJECT_LIMIT_PROCESS_MEMORY
        | JOB_OBJECT_LIMIT_PROCESS_TIME
        | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
    info.BasicLimitInformation.ActiveProcessLimit = 1;
    info.BasicLimitInformation.PerProcessUserTimeLimit = (limits.cpu_seconds * 10_000_000) as i64;
    info.ProcessMemoryLimit = limits.private_memory_bytes;
    // SAFETY: correct structure/class pairing, complete size and live job handle.
    checked(unsafe {
        SetInformationJobObject(
            raw(&job),
            JobObjectExtendedLimitInformation,
            (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    })?;
    Ok(job)
}
fn command_line(executable: &Path, args: &[impl AsRef<OsStr>]) -> io::Result<Vec<u16>> {
    let mut text = vec![34u16];
    let path = wide(executable)?;
    if path.contains(&34) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid executable",
        ));
    }
    text.extend_from_slice(&path[..path.len() - 1]);
    text.push(34);
    for arg in args {
        let arg = wide(arg)?;
        text.extend([32, 34]);
        let mut slashes = 0;
        for ch in &arg[..arg.len() - 1] {
            if *ch == 92 {
                slashes += 1;
                continue;
            }
            text.extend(std::iter::repeat_n(
                92,
                slashes * if *ch == 34 { 2 } else { 1 },
            ));
            slashes = 0;
            if *ch == 34 {
                text.push(92);
            }
            text.push(*ch);
        }
        text.extend(std::iter::repeat_n(92, slashes * 2));
        text.push(34);
    }
    text.push(0);
    Ok(text)
}

pub struct Child {
    pub stdin: Option<File>,
    pub stdout: Option<File>,
    process: OwnedHandle,
    pid: u32,
    cpu_limit_100ns: u64,
    job: OwnedHandle,
    _profile: Profile,
    _image: TempDir,
}
impl Child {
    pub fn spawn(
        executable: &Path,
        args: &[impl AsRef<OsStr>],
        limits: Limits,
    ) -> io::Result<Self> {
        limits.validate()?;
        if !executable.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "regular executable required",
            ));
        }
        let profile = Profile::new()?;
        let image = tempfile::Builder::new()
            .prefix("cube-play-image-")
            .tempdir()?;
        let binary = image.path().join("play-helper.exe");
        std::fs::copy(executable, &binary)?;
        profile.read_execute(image.path())?;
        profile.read_execute(&binary)?;
        let job = job(limits)?;
        let (input_read, input_write) = pipe()?;
        let (output_read, output_write) = pipe()?;
        // SAFETY: parent-only ends cannot leak into the child or sibling launches.
        unsafe {
            checked(SetHandleInformation(
                raw(&input_write),
                HANDLE_FLAG_INHERIT,
                0,
            ))?;
            checked(SetHandleInformation(
                raw(&output_read),
                HANDLE_FLAG_INHERIT,
                0,
            ))?;
        }
        let mut handles = [raw(&input_read), raw(&output_write)];
        let mut capabilities = SECURITY_CAPABILITIES {
            AppContainerSid: profile.sid,
            Capabilities: ptr::null_mut(),
            CapabilityCount: 0,
            Reserved: 0,
        };
        let mut attributes = Attributes::new()?;
        attributes.set(
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
            &mut capabilities,
        )?;
        attributes.set(PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize, &mut handles)?;
        // SAFETY: STARTUPINFOEXW/PROCESS_INFORMATION have valid zero defaults before fields are set.
        let mut startup: STARTUPINFOEXW = unsafe { zeroed() };
        startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput = raw(&input_read);
        startup.StartupInfo.hStdOutput = raw(&output_write);
        startup.StartupInfo.hStdError = raw(&output_write);
        startup.lpAttributeList = attributes.pointer();
        // SAFETY: Win32 fills a fresh process/thread handle pair on success.
        let mut info: PROCESS_INFORMATION = unsafe { zeroed() };
        let app = wide(&binary)?;
        let cwd = wide(image.path())?;
        let mut line = command_line(&binary, args)?;
        let env = super::environment::block()?;
        // SAFETY: explicit application path, bounded initialized attributes, empty-capability token,
        // exact handle list, trusted minimal environment; execution starts suspended.
        checked(unsafe {
            CreateProcessW(
                app.as_ptr(),
                line.as_mut_ptr(),
                ptr::null(),
                ptr::null(),
                1,
                EXTENDED_STARTUPINFO_PRESENT
                    | CREATE_SUSPENDED
                    | CREATE_NO_WINDOW
                    | CREATE_UNICODE_ENVIRONMENT,
                env.as_ptr().cast(),
                cwd.as_ptr(),
                &startup.StartupInfo,
                &mut info,
            )
        })
        .map_err(|e| io::Error::new(e.kind(), format!("CreateProcessW: {e}")))?;
        let process = owned(info.hProcess)?;
        let thread = owned(info.hThread)?;
        let child = Self {
            stdin: Some(File::from(input_write)),
            stdout: Some(File::from(output_read)),
            process,
            pid: info.dwProcessId,
            cpu_limit_100ns: limits.cpu_seconds * 10_000_000,
            job,
            _profile: profile,
            _image: image,
        };
        // SAFETY: no worker instruction executes until assignment to the required kill-on-close job.
        checked(unsafe { AssignProcessToJobObject(raw(&child.job), raw(&child.process)) })?;
        // SAFETY: the sole primary thread belongs to the contained suspended process.
        if unsafe { ResumeThread(raw(&thread)) } == u32::MAX {
            return Err(io::Error::last_os_error());
        }
        Ok(child)
    }
    pub fn id(&self) -> u32 {
        self.pid
    }
    pub fn kill(&mut self) -> io::Result<()> {
        // SAFETY: the private job contains only this worker; its handle never reaches untrusted code.
        let result = checked(unsafe { TerminateJobObject(raw(&self.job), 1) });
        // SAFETY: an assignment failure must also terminate the still-suspended owned process.
        unsafe {
            TerminateProcess(raw(&self.process), 1);
        }
        result
    }
    fn cpu_time_100ns(&self) -> io::Result<u64> {
        let mut created = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut ended = created;
        let mut kernel = created;
        let mut user = created;
        // SAFETY: the exact owned process handle has query rights; all outputs are distinct live FILETIMEs.
        checked(unsafe {
            GetProcessTimes(
                raw(&self.process),
                &mut created,
                &mut ended,
                &mut kernel,
                &mut user,
            )
        })?;
        let ticks =
            |time: FILETIME| (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime);
        ticks(user)
            .checked_add(ticks(kernel))
            .ok_or_else(|| io::Error::other("invalid native CPU accounting"))
    }
    /// Poll termination and enforce total sampled CPU in addition to the kernel's periodic job check.
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        // SAFETY: the live owned process handle is waitable and never closed during the call.
        match unsafe { WaitForSingleObject(raw(&self.process), 0) } {
            WAIT_TIMEOUT => {
                if self.cpu_time_100ns()? >= self.cpu_limit_100ns {
                    self.kill()?;
                    return self.wait().map(Some);
                }
                Ok(None)
            }
            WAIT_OBJECT_0 => self.status().map(Some),
            _ => Err(io::Error::last_os_error()),
        }
    }
    fn status(&self) -> io::Result<ExitStatus> {
        let mut code = 0;
        // SAFETY: valid process handle and output pointer after process termination.
        checked(unsafe { GetExitCodeProcess(raw(&self.process), &mut code) })?;
        Ok(ExitStatus::from_raw(code))
    }
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        // SAFETY: the owning process handle stays valid while waiting for termination.
        if unsafe { WaitForSingleObject(raw(&self.process), INFINITE) } != WAIT_OBJECT_0 {
            return Err(io::Error::last_os_error());
        }
        self.status()
    }
}
impl Drop for Child {
    fn drop(&mut self) {
        let _ = self.kill();
        let _ = self.wait();
    }
}
