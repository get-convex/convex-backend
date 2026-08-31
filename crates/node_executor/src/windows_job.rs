use std::{
    io,
    mem::size_of,
    os::windows::{
        io::{
            AsRawHandle,
            FromRawHandle,
            OwnedHandle,
        },
        process::CommandExt,
    },
    process::{
        Child,
        Command,
    },
    ptr,
};

use windows_sys::Win32::{
    Foundation::INVALID_HANDLE_VALUE,
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot,
            Thread32First,
            Thread32Next,
            TH32CS_SNAPTHREAD,
            THREADENTRY32,
        },
        JobObjects::{
            AssignProcessToJobObject,
            CreateJobObjectW,
            JobObjectExtendedLimitInformation,
            SetInformationJobObject,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        },
        Threading::{
            OpenThread,
            ResumeThread,
            CREATE_SUSPENDED,
            THREAD_SUSPEND_RESUME,
        },
    },
};
#[cfg(test)]
use windows_sys::Win32::{
    Foundation::STILL_ACTIVE,
    System::Threading::{
        GetExitCodeProcess,
        OpenProcess,
        TerminateProcess,
        PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_TERMINATE,
    },
};

/// Owns a Windows Job Object that terminates its assigned process tree when
/// this backend closes its final job handle, including abrupt termination.
pub(crate) struct KillOnCloseJob {
    handle: OwnedHandle,
}

impl KillOnCloseJob {
    pub(crate) fn new() -> io::Result<Self> {
        // SAFETY: Both optional pointer arguments are null, so Windows creates
        // an unnamed job with the caller's default security descriptor.
        let raw_handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if raw_handle.is_null() {
            return Err(io::Error::last_os_error());
        }

        // SAFETY: CreateJobObjectW returned a new owned handle on success.
        let handle = unsafe { OwnedHandle::from_raw_handle(raw_handle) };
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        // SAFETY: The handle is a live job handle, and `limits` points to a
        // correctly sized value for JobObjectExtendedLimitInformation.
        let configured = unsafe {
            SetInformationJobObject(
                handle.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                ptr::from_ref(&limits).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if configured == 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Self { handle })
    }

    pub(crate) fn assign(&self, child: &Child) -> io::Result<()> {
        // SAFETY: Both handles are live for this call. The job is configured
        // before assignment, so closing its final handle owns child teardown.
        let assigned =
            unsafe { AssignProcessToJobObject(self.handle.as_raw_handle(), child.as_raw_handle()) };
        if assigned == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Starts a process suspended, assigns it to this kill-on-close job, and
    /// only then lets its primary thread run. This closes the orphan window
    /// between process creation and job assignment.
    pub(crate) fn spawn_assigned(&self, command: &mut Command) -> io::Result<Child> {
        command.creation_flags(CREATE_SUSPENDED);
        let mut child = command.spawn()?;
        if let Err(error) = self
            .assign(&child)
            .and_then(|()| resume_process(child.id()))
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        Ok(child)
    }
}

fn resume_process(pid: u32) -> io::Result<()> {
    // SAFETY: The snapshot handle is owned locally and contains a point-in-time
    // list of system threads.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: CreateToolhelp32Snapshot returned an owned handle.
    let snapshot = unsafe { OwnedHandle::from_raw_handle(snapshot) };
    let mut entry = THREADENTRY32 {
        dwSize: size_of::<THREADENTRY32>() as u32,
        ..Default::default()
    };
    // SAFETY: snapshot and entry are live for the enumeration calls.
    let mut found = unsafe { Thread32First(snapshot.as_raw_handle(), &mut entry) } != 0;
    while found {
        if entry.th32OwnerProcessID == pid {
            // SAFETY: OpenThread returns an owned handle when the thread still
            // exists and grants the requested resume right.
            let raw_thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
            if raw_thread.is_null() {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: OpenThread returned an owned handle.
            let thread = unsafe { OwnedHandle::from_raw_handle(raw_thread) };
            // SAFETY: The handle was opened with THREAD_SUSPEND_RESUME.
            if unsafe { ResumeThread(thread.as_raw_handle()) } == u32::MAX {
                return Err(io::Error::last_os_error());
            }
            return Ok(());
        }
        // SAFETY: snapshot and entry remain valid across enumeration.
        found = unsafe { Thread32Next(snapshot.as_raw_handle(), &mut entry) } != 0;
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("suspended process {pid} had no resumable thread"),
    ))
}

#[cfg(test)]
pub(crate) fn is_process_running(pid: u32) -> bool {
    // SAFETY: OpenProcess returns a new owned handle when the PID exists.
    let raw_handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if raw_handle.is_null() {
        return false;
    }
    // SAFETY: OpenProcess returned an owned process handle.
    let handle = unsafe { OwnedHandle::from_raw_handle(raw_handle) };
    let mut exit_code = 0;
    // SAFETY: The handle is live and exit_code is writable for this call.
    let succeeded = unsafe { GetExitCodeProcess(handle.as_raw_handle(), &mut exit_code) };
    succeeded != 0 && exit_code == STILL_ACTIVE as u32
}

#[cfg(test)]
pub(crate) fn terminate_process(pid: u32) {
    // SAFETY: Any returned handle is owned locally and closed by OwnedHandle.
    let raw_handle = unsafe { OpenProcess(PROCESS_TERMINATE, 0, pid) };
    if raw_handle.is_null() {
        return;
    }
    // SAFETY: OpenProcess returned an owned process handle.
    let handle = unsafe { OwnedHandle::from_raw_handle(raw_handle) };
    // SAFETY: The handle was opened with PROCESS_TERMINATE.
    let _ = unsafe { TerminateProcess(handle.as_raw_handle(), 1) };
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        fs,
        process::Command,
        thread,
        time::{
            Duration,
            Instant,
        },
    };

    use tempfile::TempDir;

    use super::*;

    const JOB_OWNER_HELPER: &str = "CONVEX_TEST_JOB_OWNER_HELPER";
    const CHILD_PID_FILE: &str = "CONVEX_TEST_CHILD_PID_FILE";

    #[test]
    fn job_owner_helper() {
        if env::var_os(JOB_OWNER_HELPER).is_none() {
            return;
        }

        let pid_file = env::var_os(CHILD_PID_FILE).expect("missing child PID file");
        let job = KillOnCloseJob::new().expect("create job");
        let mut command = Command::new("powershell.exe");
        command.args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Start-Sleep -Seconds 300",
        ]);
        let child = job
            .spawn_assigned(&mut command)
            .expect("spawn assigned child");
        fs::write(pid_file, child.id().to_string()).expect("publish child PID");

        let _job = job;
        let _child = child;
        loop {
            thread::sleep(Duration::from_secs(60));
        }
    }

    #[test]
    fn force_killing_job_owner_terminates_assigned_process() {
        let temp_dir = TempDir::new().expect("create test directory");
        let pid_file = temp_dir.path().join("child.pid");
        let mut owner = Command::new(env::current_exe().expect("locate test executable"))
            .args([
                "--exact",
                "windows_job::tests::job_owner_helper",
                "--nocapture",
            ])
            .env(JOB_OWNER_HELPER, "1")
            .env(CHILD_PID_FILE, &pid_file)
            .spawn()
            .expect("spawn job owner");

        let child_pid = wait_for_child_pid(&pid_file);
        assert!(
            is_process_running(child_pid),
            "assigned child never started"
        );

        // std uses TerminateProcess on Windows. The owner's destructors do not
        // run, so only the kernel's job-handle semantics can stop the child.
        owner.kill().expect("force kill job owner");
        owner.wait().expect("reap job owner");

        let deadline = Instant::now() + Duration::from_secs(5);
        while is_process_running(child_pid) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(25));
        }
        if is_process_running(child_pid) {
            terminate_process(child_pid);
            panic!("assigned child survived forced job-owner termination");
        }
    }

    fn wait_for_child_pid(pid_file: &std::path::Path) -> u32 {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Ok(pid) = fs::read_to_string(pid_file) {
                return pid.parse().expect("valid child PID");
            }
            assert!(
                Instant::now() < deadline,
                "job owner did not publish child PID",
            );
            thread::sleep(Duration::from_millis(25));
        }
    }
}
