//! The process groups of the providers this daemon started, kept on disk so a daemon that dies
//! without stopping them (a SIGKILL, a crash) leaves a trail the next one can follow.
//!
//! A provider (Codex app-server, Claude) and everything it starts share one process group. A
//! clean stop kills the group (`conversation/process.rs`); a daemon that is killed never gets there,
//! and on Linux and macOS nothing else ends the group. Windows' job object does, so the module is
//! not built there. Each group is recorded with the start time of its leader, so a recorded id that
//! has since been given to an unrelated process is never signalled.

mod unix {
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};

    /// Two reads of one process's start time can differ by a second (the clock it counts from is
    /// derived); a difference past this is a different process.
    const START_TIME_SLACK_SECS: u64 = 2;

    /// A start time that could not be read when the group was recorded.
    const START_UNKNOWN: u64 = 0;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub(in crate::daemon) struct Entry {
        /// The process group, which is its leader's pid.
        pub group: i32,
        /// When the leader started (unix seconds), or `START_UNKNOWN`.
        pub started: u64,
    }

    #[derive(Debug, Default, Serialize, Deserialize)]
    struct File {
        groups: Vec<Entry>,
    }

    /// What a sweep did about one recorded group.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(in crate::daemon) enum Swept {
        /// The group's processes were killed.
        Killed { group: i32, processes: usize },
        /// Nothing in the group was left.
        Gone { group: i32 },
        /// The group's id belongs to another process now, or may: left alone.
        Skipped { group: i32 },
    }

    /// The groups file of one daemon; the daemon uses `enable`, the tests their own.
    pub(in crate::daemon) struct Groups {
        path: PathBuf,
    }

    /// One writer at a time: a provider's start and stop come from different threads.
    static LOCK: Mutex<()> = Mutex::new(());
    static ENABLED: OnceLock<Groups> = OnceLock::new();

    /// Start keeping the record in this file. The daemon calls it once it holds its socket.
    pub(in crate::daemon) fn enable(path: PathBuf) {
        let _ = ENABLED.set(Groups::at(path));
    }

    /// A provider's group started: write it down, if the record is on.
    pub(in crate::daemon) fn record(group: i32) {
        if let Some(groups) = ENABLED.get() {
            groups.record(group);
        }
    }

    /// A provider's group was stopped and reaped: strike it, if the record is on.
    pub(in crate::daemon) fn forget(group: i32) {
        if let Some(groups) = ENABLED.get() {
            groups.forget(group);
        }
    }

    /// Stop what a daemon that did not stop its providers left, and empty the record.
    pub(in crate::daemon) fn sweep_previous() -> Vec<Swept> {
        ENABLED.get().map_or_else(Vec::new, Groups::sweep)
    }

    impl Groups {
        pub(in crate::daemon) fn at(path: PathBuf) -> Self {
            Self { path }
        }

        /// A group id that is safe to signal as `-group`: never 0, 1 or a negative, which would
        /// reach every process.
        pub(in crate::daemon) fn safe_group(group: i32) -> bool {
            group > 1
        }

        /// Write a provider's group down with its leader's start time.
        pub(in crate::daemon) fn record(&self, group: i32) {
            self.record_started(group, start_time(group).unwrap_or(START_UNKNOWN));
        }

        pub(in crate::daemon) fn record_started(&self, group: i32, started: u64) {
            if !Self::safe_group(group) {
                return;
            }
            let _held = LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let mut file = self.read();
            file.groups.retain(|entry| entry.group != group);
            file.groups.push(Entry { group, started });
            self.write(&file);
        }

        pub(in crate::daemon) fn forget(&self, group: i32) {
            let _held = LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let mut file = self.read();
            let before = file.groups.len();
            file.groups.retain(|entry| entry.group != group);
            if file.groups.len() != before {
                self.write(&file);
            }
        }

        #[cfg(test)]
        pub(in crate::daemon) fn entries(&self) -> Vec<Entry> {
            let _held = LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            self.read().groups
        }

        /// Kill what each recorded group still holds, unless its id may belong to another process
        /// now, and empty the record. A record that cannot be read is an empty one.
        pub(in crate::daemon) fn sweep(&self) -> Vec<Swept> {
            let _held = LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let recorded = self.read().groups;
            let mut swept = Vec::new();
            if !recorded.is_empty() {
                let mut system = sysinfo::System::new();
                system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
                for entry in recorded.iter().filter(|entry| Self::safe_group(entry.group)) {
                    swept.push(sweep_one(&system, entry));
                }
            }
            self.write(&File::default());
            swept
        }

        fn read(&self) -> File {
            std::fs::read_to_string(&self.path)
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or_default()
        }

        fn write(&self, file: &File) {
            let written = serde_json::to_vec(file)
                .map_err(std::io::Error::other)
                .and_then(|bytes| crate::config::write_atomic(&self.path, bytes));
            if let Err(error) = written {
                tracing::warn!("provider groups file {}: {error}", self.path.display());
            }
        }
    }

    /// Kill one recorded group, or say why it was left.
    fn sweep_one(system: &sysinfo::System, entry: &Entry) -> Swept {
        let group = entry.group;
        let members = members_of(system, group);
        if members == 0 {
            return Swept::Gone { group };
        }
        let leader = sysinfo::Pid::from_u32(u32::try_from(group).unwrap_or(0));
        // A live process with the group's id: the provider itself, or another process that got
        // the id after the group ended (which the kernel does not allow while members remain,
        // but a recorded start time costs nothing to check).
        if in_group(group, group)
            && let Some(leader) = system.process(leader)
        {
            let same = entry.started != START_UNKNOWN
                && leader.start_time().abs_diff(entry.started) <= START_TIME_SLACK_SECS;
            if !same {
                tracing::info!("provider group {group} is left alone: its leader may be another process");
                return Swept::Skipped { group };
            }
        }
        // SAFETY: `safe_group` made `group` greater than 1, so `-group` names one group, never all.
        let result = unsafe { libc::kill(-group, libc::SIGKILL) };
        if result != 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
            tracing::warn!("provider group {group}: {}", std::io::Error::last_os_error());
        }
        tracing::info!("stopped {members} process(es) a previous daemon left in provider group {group}");
        Swept::Killed { group, processes: members }
    }

    fn in_group(pid: i32, group: i32) -> bool {
        // SAFETY: getpgid only reads; an unknown pid is an error value.
        unsafe { libc::getpgid(pid) == group }
    }

    fn members_of(system: &sysinfo::System, group: i32) -> usize {
        system
            .processes()
            .keys()
            .filter(|pid| i32::try_from(pid.as_u32()).is_ok_and(|pid| in_group(pid, group)))
            .count()
    }

    fn start_time(pid: i32) -> Option<u64> {
        let pid = sysinfo::Pid::from_u32(u32::try_from(pid).ok()?);
        let mut system = sysinfo::System::new();
        system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
        system.process(pid).map(sysinfo::Process::start_time)
    }
}

pub(super) use unix::{enable, forget, record, sweep_previous};
#[cfg(test)]
pub(super) use unix::{Groups, Swept};

#[cfg(all(test, unix))]
mod tests {
    use super::unix::{Groups, Swept};
    use std::os::unix::process::CommandExt;
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    fn groups() -> (Groups, crate::daemon::persist::ScratchDir) {
        let dir = crate::daemon::persist::scratch_dir();
        std::fs::create_dir_all(dir.path()).unwrap();
        (Groups::at(dir.path().join("providers")), dir)
    }

    /// A process that leads its own group, as a provider does, and sleeps.
    fn leader() -> Child {
        Command::new("sleep")
            .arg("300")
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .expect("start a sleeping group leader")
    }

    fn pid_of(child: &Child) -> i32 {
        i32::try_from(child.id()).unwrap()
    }

    fn started(pid: i32) -> u64 {
        let pid = sysinfo::Pid::from_u32(u32::try_from(pid).unwrap());
        let mut system = sysinfo::System::new();
        system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
        system.process(pid).expect("the process runs").start_time()
    }

    fn alive(pid: i32) -> bool {
        let pid = sysinfo::Pid::from_u32(u32::try_from(pid).unwrap());
        let mut system = sysinfo::System::new();
        system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
        system
            .process(pid)
            .is_some_and(|p| p.status() != sysinfo::ProcessStatus::Zombie)
    }

    fn gone_within(pid: i32, secs: u64) -> bool {
        let until = Instant::now() + Duration::from_secs(secs);
        while alive(pid) && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(20));
        }
        !alive(pid)
    }

    fn reap(mut child: Child) {
        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    fn a_recorded_group_whose_leader_still_runs_is_killed() {
        let (groups, _dir) = groups();
        let child = leader();
        let pid = pid_of(&child);
        groups.record(pid);
        assert_eq!(groups.entries().len(), 1);

        let swept = groups.sweep();

        assert_eq!(swept, [Swept::Killed { group: pid, processes: 1 }]);
        assert!(gone_within(pid, 5), "the provider outlived the sweep");
        reap(child);
        assert!(groups.entries().is_empty(), "the record is empty after a sweep");
    }

    #[test]
    fn what_a_dead_leader_left_in_its_group_is_killed() {
        let (groups, _dir) = groups();
        // The leader starts a child that keeps the group and then exits, as a provider that died
        // leaves its tools.
        let mut shell = Command::new("sh")
            .args(["-c", "sleep 300 & echo $!"])
            .process_group(0)
            .stdout(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()
            .expect("start a shell that leaves a child");
        let mut line = String::new();
        std::io::BufRead::read_line(&mut std::io::BufReader::new(shell.stdout.take().unwrap()), &mut line).unwrap();
        let orphan: i32 = line.trim().parse().expect("the child's pid");
        let group = pid_of(&shell);
        shell.wait().unwrap();
        assert!(alive(orphan));
        groups.record_started(group, 1);

        let swept = groups.sweep();

        assert_eq!(swept, [Swept::Killed { group, processes: 1 }]);
        assert!(gone_within(orphan, 5), "the orphan outlived the sweep");
    }

    #[test]
    fn a_group_id_that_another_process_now_has_is_left_alone() {
        let (groups, _dir) = groups();
        let child = leader();
        let pid = pid_of(&child);
        // Recorded as started long before this process: so this is not the one that was recorded.
        groups.record_started(pid, started(pid) + 1_000);

        let swept = groups.sweep();

        assert_eq!(swept, [Swept::Skipped { group: pid }]);
        assert!(alive(pid), "an unrelated process was killed");
        reap(child);
        assert!(groups.entries().is_empty(), "the record is emptied all the same");
    }

    #[test]
    fn a_start_time_that_could_not_be_read_never_kills_a_live_leader() {
        let (groups, _dir) = groups();
        let child = leader();
        let pid = pid_of(&child);
        groups.record_started(pid, 0);

        assert_eq!(groups.sweep(), [Swept::Skipped { group: pid }]);
        assert!(alive(pid));
        reap(child);
    }

    #[test]
    fn a_group_with_nothing_left_is_only_forgotten() {
        let (groups, _dir) = groups();
        let mut child = leader();
        let pid = pid_of(&child);
        groups.record(pid);
        child.kill().unwrap();
        child.wait().unwrap();

        assert_eq!(groups.sweep(), [Swept::Gone { group: pid }]);
        assert!(groups.entries().is_empty());
    }

    #[test]
    fn a_stopped_group_is_struck_and_the_others_stay() {
        let (groups, _dir) = groups();
        let (a, b) = (leader(), leader());
        groups.record(pid_of(&a));
        groups.record(pid_of(&b));
        groups.forget(pid_of(&a));
        assert_eq!(groups.entries().iter().map(|e| e.group).collect::<Vec<_>>(), [pid_of(&b)]);
        groups.forget(pid_of(&b));
        assert!(groups.entries().is_empty());
        reap(a);
        reap(b);
    }

    #[test]
    fn a_group_is_recorded_once_with_its_leaders_start_time() {
        let (groups, _dir) = groups();
        let child = leader();
        let pid = pid_of(&child);
        groups.record(pid);
        groups.record(pid);
        let entries = groups.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].group, pid);
        assert!(entries[0].started.abs_diff(started(pid)) <= 2, "{entries:?}");
        reap(child);
    }

    #[test]
    fn a_group_id_that_would_reach_every_process_is_never_signalled() {
        for unsafe_id in [i32::MIN, -5, -1, 0, 1] {
            assert!(!Groups::safe_group(unsafe_id), "{unsafe_id}");
        }
        assert!(Groups::safe_group(2));
        assert!(Groups::safe_group(4_000_000));
        let (groups, _dir) = groups();
        groups.record_started(1, 5);
        groups.record_started(0, 5);
        groups.record_started(-1, 5);
        assert_eq!(groups.sweep().len(), 0, "not even listed");
        assert!(groups.entries().is_empty());
    }

    #[test]
    fn a_missing_or_garbled_record_is_an_empty_one() {
        let (groups, dir) = groups();
        assert!(groups.entries().is_empty());
        assert!(groups.sweep().is_empty());
        std::fs::write(dir.path().join("providers"), b"\xff not json {").unwrap();
        assert!(groups.entries().is_empty());
        assert!(groups.sweep().is_empty());
        let child = leader();
        groups.record(pid_of(&child));
        assert_eq!(groups.entries().len(), 1, "a new record replaces the garbled one");
        reap(child);
    }
}
