//! Test-only process enumeration/observation; no mutation or production IPC surface.
#[cfg(windows)]
fn children(parent: u32) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
    use std::{
        io,
        mem::size_of,
        os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    };
    use windows_sys::Win32::{
        Foundation::{ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE},
        System::Diagnostics::ToolHelp::*,
    };
    // SAFETY: a read-only process snapshot; no process handles or mutation rights are requested.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot.is_null() || snapshot == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error().into());
    }
    // SAFETY: the successful snapshot is a fresh owning handle transferred exactly once.
    let snapshot = unsafe { OwnedHandle::from_raw_handle(snapshot) };
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    // SAFETY: live snapshot and initialized Win32 process entry with its exact documented size.
    if unsafe { Process32FirstW(snapshot.as_raw_handle(), &mut entry) } == 0 {
        return Err(io::Error::last_os_error().into());
    }
    let mut children = Vec::new();
    loop {
        if entry.th32ParentProcessID == parent && entry.th32ProcessID != 0 {
            children.push(entry.th32ProcessID);
        }
        // SAFETY: the same live snapshot and initialized process-entry buffer remain valid.
        if unsafe { Process32NextW(snapshot.as_raw_handle(), &mut entry) } == 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(ERROR_NO_MORE_FILES as i32) {
                return Err(error.into());
            }
            break;
        }
    }
    children.sort_unstable();
    Ok(children)
}
#[cfg(windows)]
fn watch(pid: u32) -> Result<(), Box<dyn std::error::Error>> {
    use std::{
        io::{self, Write},
        os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    };
    use windows_sys::Win32::{
        Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
    };
    // SAFETY: request only synchronization on this known live fixture process; no mutation rights.
    let process = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
    if process.is_null() {
        return Err(io::Error::last_os_error().into());
    }
    // SAFETY: transfer this fresh successful owning process handle exactly once.
    let process = unsafe { OwnedHandle::from_raw_handle(process) };
    // SAFETY: a zero-time read-only wait validates liveness of the exact held process object.
    if unsafe { WaitForSingleObject(process.as_raw_handle(), 0) } != WAIT_TIMEOUT {
        return Err(io::Error::other("worker already exited").into());
    }
    println!("watching");
    io::stdout().flush()?;
    // SAFETY: bounded wait on the same owning handle; PID reuse cannot satisfy it.
    if unsafe { WaitForSingleObject(process.as_raw_handle(), 5000) } != WAIT_OBJECT_0 {
        return Err(io::Error::other("owned worker was not reaped").into());
    }
    Ok(())
}
#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let (observe, id) = match args.as_slice() {
        [pid] => (false, pid),
        [mode, pid] if mode == "watch" => (true, pid),
        _ => return Err("expected owner PID or watch PID".into()),
    };
    let id = id.parse::<u32>()?;
    if id == 0 {
        return Err("invalid process identity".into());
    }
    if observe {
        return watch(id);
    }
    let children = children(id)?;
    println!(
        "[{}]",
        children
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",")
    );
    Ok(())
}
#[cfg(not(windows))]
fn main() {
    eprintln!("Windows-only qualification fixture");
    std::process::exit(1);
}
