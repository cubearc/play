//! Native contextual CLI process and PTY ownership.
use crate::Result;
use crate::workspace::{Context, RunRequest};
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;
use uuid::Uuid;

#[cfg(target_os = "macos")]
mod macos;

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Open {
        context: Context,
        #[serde(deserialize_with = "Option::deserialize")]
        selection: Option<RunRequest>,
        columns: u16,
        rows: u16,
    },
    Read {
        context: Context,
        session_id: Uuid,
    },
    Write {
        context: Context,
        session_id: Uuid,
        input: String,
    },
    Resize {
        context: Context,
        session_id: Uuid,
        columns: u16,
        rows: u16,
    },
    Close {
        context: Context,
        session_id: Uuid,
    },
}

#[derive(Default)]
pub struct Manager {
    #[cfg(target_os = "macos")]
    session: Option<macos::Session>,
}
impl Manager {
    pub fn handle(
        &mut self,
        command: Command,
        executable: Option<&Path>,
        store: &mut crate::workspace::Manager,
    ) -> Result<Value> {
        #[cfg(target_os = "macos")]
        {
            if let Command::Open {
                context,
                selection,
                columns,
                rows,
            } = command
            {
                store.authorize_context(&context)?;
                if self.session.as_ref().is_some_and(macos::Session::is_open) {
                    return Err("terminal_busy");
                }
                let session = macos::Session::open(
                    executable.ok_or("native_policy_unavailable")?,
                    context,
                    selection,
                    columns,
                    rows,
                )?;
                self.session = Some(session);
            } else {
                let session = self.session.as_mut().ok_or("terminal_closed")?;
                session.handle(command, store)?;
            }
            self.session.as_mut().ok_or("terminal_closed")?.view()
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (command, executable, store);
            Err("native_policy_unavailable")
        }
    }
    pub fn poll(&mut self) -> Result<Option<crate::protocol::Command>> {
        #[cfg(target_os = "macos")]
        {
            if let Some(session) = &mut self.session {
                return session.poll();
            }
        }
        Ok(None)
    }
    pub fn reply(&mut self, response: Value) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            if let Some(session) = &mut self.session {
                return session.reply(response);
            }
        }
        let _ = response;
        Err("terminal_closed")
    }
    pub fn close(&mut self) {
        #[cfg(target_os = "macos")]
        {
            self.session.take();
        }
    }
}

pub fn worker() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::process::CommandExt;
        crate::native::restrict()?;
        let executable = std::env::current_exe().map_err(|_| "terminal_unavailable")?;
        let cli = executable
            .parent()
            .ok_or("terminal_unavailable")?
            .join("cube");
        let _ = std::process::Command::new(cli)
            .args(["play", "session"])
            .env_clear()
            .exec();
        Err("terminal_unavailable")
    }
    #[cfg(not(target_os = "macos"))]
    Err("native_policy_unavailable")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn opening_requires_an_explicit_selection_and_refuses_authority_fields() {
        let context = json!({"workspace_id":Uuid::new_v4(),"exploration_id":Uuid::new_v4(),"stage_id":Uuid::new_v4()});
        let mut open = json!({"kind":"open","context":context,"columns":80,"rows":24});
        assert!(serde_json::from_value::<Command>(open.clone()).is_err());
        open["selection"] = Value::Null;
        assert!(serde_json::from_value::<Command>(open.clone()).is_ok());
        open["executable"] = json!("/bin/sh");
        assert!(serde_json::from_value::<Command>(open).is_err());
    }
}
