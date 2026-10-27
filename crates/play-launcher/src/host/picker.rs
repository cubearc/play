//! System folder authorization; browser input never supplies a host path.
use crate::{
    Error, Result,
    process::{self, HostProcess},
    state,
};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

pub(super) fn choose(
    home: &Path,
    new: bool,
    cancelled: Arc<AtomicBool>,
) -> Result<Option<PathBuf>> {
    let (executable, args) = command(home, new)?;
    if std::env::var_os("CUBE_PLAY_PICKER").is_none() {
        trusted_dialog(home, &executable, &cancelled)?;
    }
    let output = HostProcess {
        executable,
        cancelled,
    }
    .run(
        &args.iter().map(OsString::as_os_str).collect::<Vec<_>>(),
        home,
        Duration::from_secs(300),
    )?;
    if !output.success {
        return Err(Error::new(
            "picker_unavailable",
            "The system folder dialog was cancelled or unavailable. Retry the selection; on Linux install zenity in a desktop session, or open a directory with cube play.",
        ));
    }
    if output.stdout.len() > 16384 {
        return Err(Error::new(
            "unsupported_path",
            "The selected folder path exceeds its limit",
        ));
    }
    let value = std::str::from_utf8(&output.stdout).map_err(|_| {
        Error::new(
            "unsupported_path",
            "The system folder dialog must return a Unicode path",
        )
    })?;
    let value = value.strip_suffix('\n').unwrap_or(value);
    let value = value.strip_suffix('\r').unwrap_or(value);
    if value.is_empty() {
        return Ok(None);
    }
    let path = PathBuf::from(value);
    if !path.is_absolute() || value.contains('\0') || !path.is_dir() {
        return Err(Error::new(
            "invalid_path",
            "Choose an existing local directory in the system dialog",
        ));
    }
    Ok(Some(path.canonicalize().map_err(Error::io)?))
}
fn trusted_dialog(home: &Path, executable: &Path, cancelled: &Arc<AtomicBool>) -> Result<()> {
    if state::recent_workspaces(home, cancelled)?
        .iter()
        .any(|workspace| state::contains_path(&workspace.root, executable))
    {
        return Err(Error::new(
            "picker_unavailable",
            "The folder dialog executable is inside an investigation directory. Use a trusted system installation, or open the directory with cube play.",
        ));
    }
    Ok(())
}
fn command(home: &Path, new: bool) -> Result<(PathBuf, Vec<OsString>)> {
    if let Some(path) = std::env::var_os("CUBE_PLAY_PICKER") {
        return Ok((
            process::executable("", Some(path.into()), Some(home))?,
            vec![if new { "new" } else { "open" }.into()],
        ));
    }
    #[cfg(target_os = "macos")]
    {
        let script = if new {
            "try\nreturn POSIX path of (choose folder with prompt \"Choose the parent folder for a new Cube Play workspace\")\non error number -128\nreturn \"\"\nend try"
        } else {
            "try\nreturn POSIX path of (choose folder with prompt \"Open a local Cube Play workspace\")\non error number -128\nreturn \"\"\nend try"
        };
        Ok((
            PathBuf::from("/usr/bin/osascript"),
            vec!["-e".into(), script.into()],
        ))
    }
    #[cfg(windows)]
    {
        let system = std::env::var_os("SystemRoot").ok_or_else(|| {
            Error::new("picker_unavailable", "Windows system folder is unavailable")
        })?;
        let path = PathBuf::from(system).join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let executable = process::executable("", Some(path), Some(home))?;
        let script = "[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); Add-Type -AssemblyName System.Windows.Forms; $d=[System.Windows.Forms.FolderBrowserDialog]::new(); $d.Description='Choose a local Cube Play workspace directory'; if($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK){[Console]::WriteLine($d.SelectedPath)}; $d.Dispose()";
        let _ = new;
        Ok((
            executable,
            vec![
                "-NoProfile".into(),
                "-STA".into(),
                "-Command".into(),
                script.into(),
            ],
        ))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let executable=process::executable("zenity",None,Some(home)).map_err(|_|Error::new("picker_unavailable","Install zenity in a desktop session for system folder selection, or open a directory with cube play"))?;
        Ok((
            executable,
            vec![
                "--file-selection".into(),
                "--directory".into(),
                if new {
                    "--title=Choose the parent folder for a new workspace"
                } else {
                    "--title=Open a Cube Play workspace"
                }
                .into(),
            ],
        ))
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (home, new);
        Err(Error::new(
            "picker_unavailable",
            "System folder selection is unsupported on this platform; open a directory with cube play",
        ))
    }
}
pub(super) fn leaf(name: &str) -> Result<()> {
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if name.trim().is_empty()
        || name.len() > 128
        || name == "."
        || name == ".."
        || name.ends_with(['.', ' '])
        || name.chars().any(|c| {
            c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
        || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
    {
        return Err(Error::new(
            "invalid_name",
            "Use a portable folder name of 1–128 UTF-8 bytes without separators, control characters or reserved device names",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_dialog_must_stay_outside_registered_investigations() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let home = root.join("home");
        state::private_directory(&home).unwrap();
        let source = root.join("untrusted");
        std::fs::create_dir(&source).unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        state::workspace(&home, &source, true, &cancelled).unwrap();
        let executable = source.join("zenity");
        std::fs::write(&executable, b"untrusted program").unwrap();
        assert!(trusted_dialog(&home, &executable, &cancelled).is_err());
        assert!(trusted_dialog(&home, &root.join("system-dialog"), &cancelled).is_ok());
    }
}
