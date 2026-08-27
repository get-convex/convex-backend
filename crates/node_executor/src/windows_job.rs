use std::{
    io,
    mem::size_of,
    os::windows::io::{
        AsRawHandle,
        FromRawHandle,
        OwnedHandle,
    },
    process::Child,
    ptr,
};

use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject,
    CreateJobObjectW,
    JobObjectExtendedLimitInformation,
    SetInformationJobObject,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
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
        let child = Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 300",
            ])
            .spawn()
            .expect("spawn child");
        job.assign(&child).expect("assign child");
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
