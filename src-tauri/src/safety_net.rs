//! Logon safety net shared by the two things that change Windows and must be undone after a crash:
//! muted notifications (`crate::shields`) and the warm screen colours (`crate::tint`).
//!
//! Both persist a restore file before they change Windows. If the app dies before it restores them, the next
//! start repairs them from that file, and the per-user `RunOnce` entry (`FocusMode.exe --restore-shields`) does
//! it at the next logon even if the app is never started again. There is only one `RunOnce` value, so it is
//! registered while **either** restore is pending and removed only when **neither** is. One mutex serializes
//! the check and the registry change, so one worker can never remove the entry the other one just armed.

use std::sync::{Arc, Mutex};

use focus_platform::logon::LogonTask;

use crate::{RESTORE_SHIELDS_ARG, lock};

/// What a pending restore repairs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pending {
    /// Windows notifications muted during focus (`shields-restore.json`).
    Notifications,
    /// Screen colours changed by the tint (`tint-restore.json`).
    Tint,
}

/// Cheap to clone. Every method touches the registry at most once; call it from a worker thread.
#[derive(Clone)]
pub struct SafetyNet {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    task: Box<dyn LogonTask>,
    notifications: bool,
    tint: bool,
}

impl Inner {
    fn flag(&mut self, pending: Pending) -> &mut bool {
        match pending {
            Pending::Notifications => &mut self.notifications,
            Pending::Tint => &mut self.tint,
        }
    }
}

impl SafetyNet {
    /// `notifications` / `tint`: a restore file of that kind exists at startup (its worker repairs it soon,
    /// and until then the other worker must not remove the logon entry).
    pub fn new(notifications: bool, tint: bool) -> Self {
        Self::with_task(
            focus_platform::logon::restore_at_logon(),
            notifications,
            tint,
        )
    }

    fn with_task(task: Box<dyn LogonTask>, notifications: bool, tint: bool) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                task,
                notifications,
                tint,
            })),
        }
    }

    /// Marks `pending` and (re-)registers the logon entry. Registering again is harmless and needed after a
    /// `--restore-shields` launch: Windows deletes the value before it runs the command.
    /// Best effort: the restore file alone still repairs it at the next start of the app.
    pub fn arm(&self, pending: Pending) {
        let mut inner = lock(&self.inner);
        *inner.flag(pending) = true;
        let exe = match std::env::current_exe() {
            Ok(exe) => exe,
            Err(err) => {
                log::warn!("cannot register the logon restore: executable path unknown: {err}");
                return;
            }
        };
        if let Err(err) = inner.task.schedule(&exe, RESTORE_SHIELDS_ARG)
            && !err.is_unsupported()
        {
            log::warn!("cannot register the logon restore: {err}");
        }
    }

    /// Clears `pending`; removes the logon entry when nothing else is pending.
    pub fn disarm(&self, pending: Pending) {
        let mut inner = lock(&self.inner);
        *inner.flag(pending) = false;
        if inner.notifications || inner.tint {
            return;
        }
        if let Err(err) = inner.task.cancel() {
            log::warn!("cannot remove the logon restore: {err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use focus_platform::PlatformError;
    use std::path::Path;

    /// Records whether the entry is registered.
    struct FakeTask(Arc<Mutex<bool>>);

    impl LogonTask for FakeTask {
        fn schedule(&mut self, _exe: &Path, args: &str) -> Result<(), PlatformError> {
            assert_eq!(args, RESTORE_SHIELDS_ARG);
            *lock(&self.0) = true;
            Ok(())
        }

        fn cancel(&mut self) -> Result<(), PlatformError> {
            *lock(&self.0) = false;
            Ok(())
        }
    }

    #[test]
    fn the_entry_stays_while_either_restore_is_pending() {
        let registered = Arc::new(Mutex::new(false));
        let net = SafetyNet::with_task(Box::new(FakeTask(Arc::clone(&registered))), false, false);

        net.arm(Pending::Notifications);
        net.arm(Pending::Tint);
        assert!(*lock(&registered));
        net.disarm(Pending::Notifications);
        assert!(*lock(&registered), "the tint still needs it");
        net.disarm(Pending::Tint);
        assert!(!*lock(&registered));
    }

    #[test]
    fn a_restore_pending_at_startup_keeps_the_entry() {
        let registered = Arc::new(Mutex::new(true));
        let net = SafetyNet::with_task(Box::new(FakeTask(Arc::clone(&registered))), false, true);
        // Shields had nothing to restore and drop a stale entry: the tint file still needs it.
        net.disarm(Pending::Notifications);
        assert!(*lock(&registered));
        net.disarm(Pending::Tint);
        assert!(!*lock(&registered));
    }
}
