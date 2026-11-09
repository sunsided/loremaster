//! The state that the request thread and the job threads share: the jobs and
//! their running agent processes, each behind a lock.
//!
//! It is apart from the processes and the pipes, so that `loom` can check all
//! orders of `finish`, `cancel` and the readers. Build with `--cfg loom` to
//! run these checks (see CONTRIBUTING.md).

use super::{Job, JobEvent, JobStatus};
use std::collections::BTreeMap;
use std::process::Child;

#[cfg(loom)]
use loom::sync::{Arc, Mutex};
#[cfg(not(loom))]
use std::sync::{Arc, Mutex};

/// What the registry needs of an agent process. A fake in the loom tests.
pub(super) trait Process {
    fn kill(&mut self);
    /// Wait for the end; true when the process succeeded.
    fn wait(&mut self) -> bool;
}

impl Process for Child {
    fn kill(&mut self) {
        let _ = Child::kill(self);
    }

    fn wait(&mut self) -> bool {
        Child::wait(self).is_ok_and(|s| s.success())
    }
}

pub(super) struct Registry<P = Child> {
    pub(super) jobs: Arc<Mutex<BTreeMap<String, Job>>>,
    children: Arc<Mutex<BTreeMap<String, P>>>,
}

impl<P> Default for Registry<P> {
    fn default() -> Self {
        Self {
            jobs: Arc::default(),
            children: Arc::default(),
        }
    }
}

impl<P> Clone for Registry<P> {
    fn clone(&self) -> Self {
        Self {
            jobs: self.jobs.clone(),
            children: self.children.clone(),
        }
    }
}

impl<P: Process> Registry<P> {
    /// Add a running job and its process.
    ///
    /// Both at once, so that a job that others can see can also be cancelled.
    /// The only place that holds both locks; it takes `children` first.
    pub(super) fn insert(&self, job: Job, child: P) {
        let id = job.id.clone();
        let mut children = self.children.lock().unwrap();
        self.jobs.lock().unwrap().insert(id.clone(), job);
        children.insert(id, child);
    }

    /// Change a job, if it is there and not cancelled. A cancelled job stays as
    /// it is, also when the agent's last output comes in after the kill.
    pub(super) fn update(&self, id: &str, f: impl FnOnce(&mut Job)) {
        if let Some(j) = self.jobs.lock().unwrap().get_mut(id)
            && j.status != JobStatus::Cancelled
        {
            f(j);
        }
    }

    pub(super) fn get(&self, id: &str) -> Option<Job> {
        self.jobs.lock().unwrap().get(id).cloned()
    }

    pub(super) fn all(&self) -> Vec<Job> {
        self.jobs.lock().unwrap().values().cloned().collect()
    }

    /// The agent's output has ended: wait for it and give the job its final status.
    /// `err` is the agent's stderr.
    pub(super) fn finish(&self, id: &str, err: &str) {
        // No child: cancel() took it and sets the status. Leave the job alone,
        // else it shows as failed (with the kill's error) until then. With the
        // child here, cancel() can no longer take it, so the status is ours.
        // The lock is free during the wait: a slow exit blocks no other job.
        let child = self.children.lock().unwrap().remove(id);
        let Some(mut child) = child else {
            return;
        };
        let success = child.wait();
        if let Some(j) = self.jobs.lock().unwrap().get_mut(id)
            && j.status == JobStatus::Running
        {
            j.status = if success && j.result.is_some() {
                JobStatus::Done
            } else {
                JobStatus::Failed
            };
            if !success && !err.trim().is_empty() {
                j.events.push(JobEvent {
                    kind: "error".into(),
                    text: err.trim().to_string(),
                });
            }
        }
    }

    /// Stop a running job. Proposals it made stay. False when it does not run.
    pub(super) fn cancel(&self, id: &str) -> bool {
        let Some(mut child) = self.children.lock().unwrap().remove(id) else {
            return false;
        };
        // Cancelled before the kill: update() then drops what the agent
        // writes while it dies, and no reader sees it failed on the way.
        if let Some(j) = self.jobs.lock().unwrap().get_mut(id) {
            j.status = JobStatus::Cancelled;
        }
        child.kill();
        child.wait();
        true
    }
}

#[cfg(all(test, loom))]
mod loom_tests {
    use super::*;
    use loom::sync::atomic::{AtomicBool, Ordering};
    use loom::thread;

    /// An agent that ends on its own with success, or fails once killed.
    /// `killed` is shared, so that a test can see when the kill began.
    #[derive(Default)]
    struct Fake {
        killed: Arc<AtomicBool>,
    }

    impl Process for Fake {
        fn kill(&mut self) {
            self.killed.store(true, Ordering::SeqCst);
        }

        fn wait(&mut self) -> bool {
            !self.killed.load(Ordering::SeqCst)
        }
    }

    fn registry() -> Registry<Fake> {
        registry_with(Fake::default())
    }

    fn registry_with(child: Fake) -> Registry<Fake> {
        let r = Registry::default();
        let mut job = super::super::tests::job();
        job.result = Some("done".into());
        r.insert(job, child);
        r
    }

    /// A cancel that wins makes the job cancelled, and a reader never sees it
    /// failed on the way. A cancel that loses leaves the job done.
    #[test]
    fn loom_cancel_races_finish() {
        loom::model(|| {
            let r = registry();
            let finisher = {
                let r = r.clone();
                thread::spawn(move || r.finish("j1", "killed"))
            };
            let reader = {
                let r = r.clone();
                thread::spawn(move || r.get("j1").unwrap())
            };
            let cancelled = r.cancel("j1");
            finisher.join().unwrap();
            let seen = reader.join().unwrap();
            let end = r.get("j1").unwrap();
            if cancelled {
                assert_eq!(end.status, JobStatus::Cancelled);
                assert!(end.events.is_empty(), "{:?}", end.events);
                assert_ne!(seen.status, JobStatus::Failed);
                assert_ne!(seen.status, JobStatus::Done);
            } else {
                assert_eq!(end.status, JobStatus::Done);
            }
        });
    }

    /// Two cancels of the same job: exactly one stops it.
    #[test]
    fn loom_one_of_two_cancels_wins() {
        loom::model(|| {
            let r = registry();
            let other = {
                let r = r.clone();
                thread::spawn(move || r.cancel("j1"))
            };
            let mine = r.cancel("j1");
            assert!(mine ^ other.join().unwrap());
            assert_eq!(r.get("j1").unwrap().status, JobStatus::Cancelled);
        });
    }

    /// The agent's last output comes in while the job is cancelled: from
    /// the kill on, an update (here a result with an error) does not reach
    /// the job, so it never shows as failed and gains no events.
    #[test]
    fn loom_cancelled_is_final() {
        loom::model(|| {
            let child = Fake::default();
            let killed = child.killed.clone();
            let r = registry_with(child);
            let reader = {
                let r = r.clone();
                thread::spawn(move || {
                    r.update("j1", |j| {
                        assert!(!killed.load(Ordering::SeqCst), "an update after the kill");
                        j.status = JobStatus::Failed;
                        j.events.push(JobEvent {
                            kind: "text".into(),
                            text: "late".into(),
                        });
                    });
                    r.finish("j1", "killed");
                })
            };
            let cancelled = r.cancel("j1");
            reader.join().unwrap();
            let end = r.get("j1").unwrap();
            if cancelled {
                assert_eq!(end.status, JobStatus::Cancelled);
            } else {
                assert_eq!(end.status, JobStatus::Failed);
            }
        });
    }

    /// A job that a reader sees can be cancelled, also while it starts.
    #[test]
    fn loom_start_races_cancel() {
        loom::model(|| {
            let r: Registry<Fake> = Registry::default();
            let canceller = {
                let r = r.clone();
                thread::spawn(move || r.get("j1").is_none() || r.cancel("j1"))
            };
            r.insert(super::super::tests::job(), Fake::default());
            assert!(
                canceller.join().unwrap(),
                "a running job without its process"
            );
        });
    }

    /// A new job starts while another finishes: neither loses the other's job.
    /// (Only `insert` holds both locks, so there is no second lock order.)
    #[test]
    fn loom_start_during_finish() {
        loom::model(|| {
            let r = registry();
            let finisher = {
                let r = r.clone();
                thread::spawn(move || r.finish("j1", ""))
            };
            let mut job = super::super::tests::job();
            job.id = "j2".into();
            r.insert(job, Fake::default());
            finisher.join().unwrap();
            assert_eq!(r.get("j1").unwrap().status, JobStatus::Done);
            assert_eq!(r.get("j2").unwrap().status, JobStatus::Running);
            assert!(r.cancel("j2"));
        });
    }
}
