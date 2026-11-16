//! A PID plus OS process start time prevents repairs after the desktop process exits.
use crate::{
    api::ErrorCode,
    util::{self, Result, error, io_error},
};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
#[derive(Serialize, Deserialize)]
struct Running {
    pid: u32,
    start_time: u64,
}
fn start_time(pid: u32) -> Option<u64> {
    let pid = Pid::from_u32(pid);
    let mut system = System::new();
    // Read only this PID's base information, never process command lines or environments.
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing(),
    );
    system
        .process(pid)
        .map(|p| p.start_time())
        .filter(|t| *t > 0)
}
pub(crate) fn check(instance: &Path) -> Result<()> {
    let path = util::join(instance, ".ooze-running-game.json")?;
    if path.is_file() {
        let running: Running = util::read_json(&path)?;
        if start_time(running.pid) == Some(running.start_time) {
            return Err(error(
                ErrorCode::Busy,
                "Minecraft is still running for this instance. Close it before installing, repairing or launching again.",
            ));
        }
        fs::remove_file(path).map_err(|_| io_error())?;
    }
    Ok(())
}
pub(crate) fn record(instance: &Path, pid: u32) -> Result<()> {
    let start_time = start_time(pid).ok_or_else(|| {
        error(
            ErrorCode::Process,
            "Couldn't record the Java process identity safely.",
        )
    })?;
    util::atomic_json(
        &util::join(instance, ".ooze-running-game.json")?,
        &Running { pid, start_time },
    )
}
pub(crate) fn clear(instance: &Path, pid: u32) -> Result<()> {
    let path = util::join(instance, ".ooze-running-game.json")?;
    if path.is_file() && util::read_json::<Running>(&path)?.pid == pid {
        fs::remove_file(path).map_err(|_| io_error())?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(windows)]
    fn detects_live_process_and_preserves_guard_until_exit() {
        let dir = tempfile::tempdir().unwrap();
        let pid = std::process::id();
        record(dir.path(), pid).unwrap();
        assert_eq!(check(dir.path()).unwrap_err().code, ErrorCode::Busy);
        clear(dir.path(), pid).unwrap();
        assert!(check(dir.path()).is_ok());
    }
}
