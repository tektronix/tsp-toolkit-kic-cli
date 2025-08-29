use std::path::PathBuf;

#[derive(Debug)]
pub struct Process {
    path: PathBuf,
    args: Vec<String>,
}
impl Process {
    pub fn new<I>(path: PathBuf, args: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        Self {
            path,
            args: args.into_iter().map(|s| s.as_ref().to_string()).collect(),
        }
    }

    pub fn exec_replace(self) -> anyhow::Result<()> {
        imp::exec_replace(&self)
    }

    #[cfg_attr(unix, allow(dead_code))]
    pub fn exec(&self) -> anyhow::Result<()> {
        let exit = std::process::Command::new(&self.path)
            .args(&self.args)
            .spawn()?
            .wait()?;
        if exit.success() {
            Ok(())
        } else {
            Err(std::io::Error::other(format!(
                "child process did not exit successfully: {}",
                self.path.display()
            ))
            .into())
        }
    }
}

#[cfg(windows)]
mod imp {

    use super::Process;
    use windows_sys::Win32::{
        Foundation::{BOOL, FALSE, TRUE},
        System::Console::SetConsoleCtrlHandler,
    };

    #[allow(clippy::missing_const_for_fn)] // This lint seems to be broken for this specific case
    unsafe extern "system" fn ctrlc_handler(_: u32) -> BOOL {
        TRUE
    }

    pub(super) fn exec_replace(process: &Process) -> anyhow::Result<()> {
        //Safety: This is an external handler that calls into the windows API. It is
        //        expected to be safe.
        unsafe {
            if SetConsoleCtrlHandler(Some(ctrlc_handler), TRUE) == FALSE {
                return Err(
                    std::io::Error::other("Unable to set Ctrl+C Handler".to_string()).into(),
                );
            }
        }

        process.exec()
    }
}

#[cfg(unix)]
mod imp {
    use crate::Process;
    use std::os::unix::process::CommandExt;

    pub(super) fn exec_replace(process: &Process) -> anyhow::Result<()> {
        let mut command = std::process::Command::new(&process.path);
        command.args(&process.args);
        Err(command.exec().into()) // Exec replaces the current application's program memory, therefore execution will
    }
}
