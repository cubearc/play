//! Start the long-lived chooser without inheriting the caller's output pipes.
use std::{ffi::OsStr, io, process::Command};
use winsafe::{co, guard::CloseHandlePiGuard};

pub(super) struct Child(CloseHandlePiGuard);

fn error(error: co::ERROR) -> io::Error {
    io::Error::from_raw_os_error(error.raw() as i32)
}

fn text(value: &OsStr) -> io::Result<&str> {
    value
        .to_str()
        .filter(|value| !value.contains('\0'))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid Windows process argument",
            )
        })
}

// Apply the Windows CRT argv rules, including quotes and trailing backslashes.
fn quote(value: &str) -> String {
    let mut result = String::from("\"");
    let mut slashes = 0;
    for character in value.chars() {
        if character == '\\' {
            slashes += 1;
            continue;
        }
        result.extend(std::iter::repeat_n(
            '\\',
            slashes * if character == '"' { 2 } else { 1 },
        ));
        if character == '"' {
            result.push('\\');
        }
        result.push(character);
        slashes = 0;
    }
    result.extend(std::iter::repeat_n('\\', slashes * 2));
    result.push('"');
    result
}

pub(super) fn spawn(command: &Command) -> io::Result<Child> {
    let arguments = std::iter::once(command.get_program())
        .chain(command.get_args())
        .map(|value| text(value).map(quote))
        .collect::<io::Result<Vec<_>>>()?
        .join(" ");
    let directory = command
        .get_current_dir()
        .map(|path| text(path.as_os_str()))
        .transpose()?;
    // Stdio::null alone still allows Rust's Windows spawn to inherit other open
    // handles. This host has no stdio and must release every caller-owned pipe.
    winsafe::CreateProcess(
        Some(text(command.get_program())?),
        Some(&arguments),
        None,
        None,
        false,
        co::CREATE::NO_WINDOW,
        &[],
        directory,
        &mut winsafe::STARTUPINFO::default(),
    )
    .map(Child)
    .map_err(error)
}

impl Child {
    pub(super) fn try_wait(&mut self) -> io::Result<Option<u32>> {
        match self
            .0
            .hProcess
            .WaitForSingleObject(Some(0))
            .map_err(error)?
        {
            co::WAIT::TIMEOUT => Ok(None),
            _ => self
                .0
                .hProcess
                .GetExitCodeProcess()
                .map(Some)
                .map_err(error),
        }
    }

    pub(super) fn kill(&mut self) -> io::Result<()> {
        self.0.hProcess.TerminateProcess(1).map_err(error)
    }

    pub(super) fn wait(&mut self) -> io::Result<()> {
        self.0
            .hProcess
            .WaitForSingleObject(None)
            .map(|_| ())
            .map_err(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_keep_literal_paths_quotes_empty_values_and_trailing_slashes() {
        assert_eq!(quote(""), "\"\"");
        assert_eq!(
            quote(r"D:\investigation $private 空"),
            "\"D:\\investigation $private 空\""
        );
        assert_eq!(quote("a\\\"b"), "\"a\\\\\\\"b\"");
        assert_eq!(quote("D:\\space path\\"), "\"D:\\space path\\\\\"");
        assert!(text(OsStr::new("invalid\0argument")).is_err());
    }
}
