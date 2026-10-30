use std::io;
use std::process::{Command, ExitStatus};
use tokio::process::Child;

pub(super) struct OwnedChild {
    pub(super) child: Child,
    tree: Tree,
    #[cfg(test)]
    fail_next_stop: bool,
}

pub(super) struct SpawnFailure {
    pub(super) error: io::Error,
    pub(super) process: Option<Box<OwnedChild>>,
    pub(super) reaped: bool,
}

impl OwnedChild {
    pub(super) async fn spawn(command: Command) -> Result<Self, SpawnFailure> {
        let runtime = tokio::runtime::Handle::current();
        let (mut owned, startup_error) = tokio::task::spawn_blocking(move || {
            let _entered = runtime.enter();
            Self::spawn_suspended(command)
        })
        .await
        .map_err(|error| SpawnFailure {
            error: io::Error::other(format!("provider launch task failed: {error}")),
            process: None,
            reaped: false,
        })?
        .map_err(|error| SpawnFailure {
            error,
            process: None,
            reaped: true,
        })?;

        if let Some(error) = startup_error {
            return match owned.stop().await {
                Ok(_) => Err(SpawnFailure {
                    error,
                    process: None,
                    reaped: true,
                }),
                Err(cleanup) => Err(SpawnFailure {
                    error: io::Error::other(format!(
                        "{error}; provider startup cleanup failed: {cleanup}"
                    )),
                    process: Some(Box::new(owned)),
                    reaped: false,
                }),
            };
        }
        Ok(owned)
    }

    fn spawn_suspended(mut command: Command) -> io::Result<(Self, Option<io::Error>)> {
        crate::proc::quiet(&mut command);
        configure(&mut command);
        let tree = Tree::new()?;
        let child = tokio::process::Command::from(command)
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| context("starting provider process", error))?;
        let mut owned = Self {
            child,
            tree,
            #[cfg(test)]
            fail_next_stop: false,
        };
        let startup_error = owned.tree.attach(&owned.child).err();
        Ok((owned, startup_error))
    }

    pub(super) async fn stop(&mut self) -> io::Result<ExitStatus> {
        #[cfg(test)]
        if std::mem::take(&mut self.fail_next_stop) {
            return Err(io::Error::other("injected provider disposal failure"));
        }
        let tree_result = self.tree.terminate();
        if let Err(error) = self.child.start_kill() {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    tree_result?;
                    return Ok(status);
                }
                Ok(None) => return Err(context("stopping provider process", error)),
                Err(wait_error) => {
                    return Err(context("checking provider process exit", wait_error));
                }
            }
        }
        let status = self
            .child
            .wait()
            .await
            .map_err(|error| context("waiting for provider process exit", error))?;
        tree_result?;
        Ok(status)
    }

    #[cfg(test)]
    pub(super) fn fail_next_stop(&mut self) {
        self.fail_next_stop = true;
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Err(error) = self.tree.terminate() {
            tracing::warn!("stopping provider process tree on disposal: {error}");
        }
    }
}

fn context(operation: &str, error: io::Error) -> io::Error {
    io::Error::new(error.kind(), format!("{operation}: {error}"))
}

#[cfg(unix)]
fn configure(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(windows)]
fn configure(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::CREATE_SUSPENDED;
    command.creation_flags(crate::proc::CREATE_NO_WINDOW | CREATE_SUSPENDED);
}

#[cfg(unix)]
struct Tree {
    group: Option<libc::pid_t>,
}

#[cfg(unix)]
impl Tree {
    fn new() -> io::Result<Self> {
        Ok(Self { group: None })
    }

    fn attach(&mut self, child: &Child) -> io::Result<()> {
        let group = child
            .id()
            .and_then(|id| libc::pid_t::try_from(id).ok())
            .filter(|&id| id > 1)
            .ok_or_else(|| io::Error::other("provider has no safe process-group identity"))?;
        self.group = Some(group);
        Ok(())
    }

    fn terminate(&mut self) -> io::Result<()> {
        let Some(group) = self.group else {
            return Ok(());
        };
        // SAFETY: process_group(0) created this group; the provider owns its lifetime.
        if unsafe { libc::kill(-group, libc::SIGKILL) } != 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(context("stopping provider process group", error));
            }
        }
        self.group = None;
        Ok(())
    }
}

#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};

#[cfg(windows)]
struct Tree {
    job: OwnedHandle,
    terminated: bool,
}

#[cfg(windows)]
impl Tree {
    fn new() -> io::Result<Self> {
        use windows_sys::Win32::System::JobObjects::{
            CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        };
        // SAFETY: creates an unnamed, non-inheritable job, transferred once to OwnedHandle.
        let raw = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if raw.is_null() {
            return Err(context("creating provider job", io::Error::last_os_error()));
        }
        // SAFETY: CreateJobObjectW returned a valid handle owned by this function.
        let job = unsafe { OwnedHandle::from_raw_handle(raw.cast()) };
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: job is live and limits has the exact layout/size required by this information class.
        let configured = unsafe {
            SetInformationJobObject(
                raw,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
        };
        if configured == 0 {
            return Err(context(
                "configuring provider job",
                io::Error::last_os_error(),
            ));
        }
        Ok(Self {
            job,
            terminated: false,
        })
    }

    fn attach(&mut self, child: &Child) -> io::Result<()> {
        use windows_sys::Win32::System::JobObjects::AssignProcessToJobObject;
        let process = child
            .raw_handle()
            .ok_or_else(|| io::Error::other("provider process handle is unavailable"))?;
        let pid = child
            .id()
            .ok_or_else(|| io::Error::other("provider process identity is unavailable"))?;
        // SAFETY: both handles remain owned and live; CREATE_SUSPENDED prevents startup descendants.
        if unsafe { AssignProcessToJobObject(self.job.as_raw_handle().cast(), process.cast()) } == 0
        {
            return Err(context(
                "assigning provider job",
                io::Error::last_os_error(),
            ));
        }
        resume_owned_thread(pid)
    }

    fn terminate(&mut self) -> io::Result<()> {
        use windows_sys::Win32::System::JobObjects::TerminateJobObject;
        if self.terminated {
            return Ok(());
        }
        // SAFETY: the unnamed job is exclusively owned by this guard.
        if unsafe { TerminateJobObject(self.job.as_raw_handle().cast(), 1) } == 0 {
            return Err(context("stopping provider job", io::Error::last_os_error()));
        }
        self.terminated = true;
        Ok(())
    }
}

#[cfg(windows)]
fn resume_owned_thread(pid: u32) -> io::Result<()> {
    use windows_sys::Win32::System::Threading::{
        GetProcessIdOfThread, OpenThread, ResumeThread, THREAD_QUERY_LIMITED_INFORMATION,
        THREAD_SUSPEND_RESUME,
    };
    let thread_id = suspended_thread(pid)?;
    // SAFETY: requests only query/resume rights for the one thread in our suspended process.
    let raw = unsafe {
        OpenThread(
            THREAD_SUSPEND_RESUME | THREAD_QUERY_LIMITED_INFORMATION,
            0,
            thread_id,
        )
    };
    if raw.is_null() {
        return Err(context(
            "opening provider startup thread",
            io::Error::last_os_error(),
        ));
    }
    // SAFETY: OpenThread returned an owned, valid handle.
    let thread = unsafe { OwnedHandle::from_raw_handle(raw.cast()) };
    // SAFETY: thread remains open, and the child process handle remains owned by the caller.
    if unsafe { GetProcessIdOfThread(thread.as_raw_handle().cast()) } != pid {
        return Err(io::Error::other(
            "provider startup thread ownership changed",
        ));
    }
    // SAFETY: the thread belongs to the suspended child, which is already contained in our job.
    match unsafe { ResumeThread(thread.as_raw_handle().cast()) } {
        1 => Ok(()),
        u32::MAX => Err(context(
            "resuming provider startup thread",
            io::Error::last_os_error(),
        )),
        _ => Err(io::Error::other(
            "provider startup thread had an unexpected suspend count",
        )),
    }
}

#[cfg(windows)]
fn suspended_thread(pid: u32) -> io::Result<u32> {
    use windows_sys::Win32::Foundation::{ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
    };
    // SAFETY: requests read-only thread metadata; no process memory or user data is read.
    let raw = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if raw == INVALID_HANDLE_VALUE {
        return Err(context(
            "finding provider startup thread",
            io::Error::last_os_error(),
        ));
    }
    // SAFETY: snapshot creation returned a valid handle, closed once at the end of this function.
    let snapshot = unsafe { OwnedHandle::from_raw_handle(raw.cast()) };
    let mut entry = THREADENTRY32 {
        dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
        ..Default::default()
    };
    let mut found = None;
    // SAFETY: entry is initialized with its size and remains valid through each enumeration call.
    let mut present = unsafe { Thread32First(snapshot.as_raw_handle().cast(), &mut entry) };
    while present != 0 {
        let owner_end = std::mem::offset_of!(THREADENTRY32, th32OwnerProcessID)
            + std::mem::size_of_val(&entry.th32OwnerProcessID);
        if (entry.dwSize as usize) < owner_end {
            return Err(io::Error::other(
                "provider thread snapshot has incomplete ownership metadata",
            ));
        }
        if entry.th32OwnerProcessID == pid {
            if found.is_some() {
                return Err(io::Error::other(
                    "provider startup has more than one thread",
                ));
            }
            found = Some(entry.th32ThreadID);
        }
        entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
        // SAFETY: snapshot remains open and entry has its documented size.
        present = unsafe { Thread32Next(snapshot.as_raw_handle().cast(), &mut entry) };
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() != Some(ERROR_NO_MORE_FILES as i32) {
        return Err(context("enumerating provider startup threads", error));
    }
    found.ok_or_else(|| io::Error::other("provider startup thread was not found"))
}
