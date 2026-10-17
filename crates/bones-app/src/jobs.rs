//! Tâches de fond de l'éditeur : liaison des poids et export tournent hors
//! du fil principal pour garder l'interface fluide. Chaque tâche réveille la
//! boucle d'événements quand elle se termine. (Les accès aux projets ont
//! leur propre fil : voir `crate::projects`.)

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};

use bones_core::editor::BindJob;
use bones_core::io::Format;
use bones_core::{Rig, Skin};
use winit::event_loop::EventLoopProxy;

use crate::UserEvent;

pub enum JobResult {
    Bound(Skin, (u64, u64)),
    /// Fichiers écrits (le principal en premier).
    Exported(Result<Vec<PathBuf>, String>),
}

pub struct Jobs {
    sender: Sender<JobResult>,
    receiver: Receiver<JobResult>,
    proxy: EventLoopProxy<UserEvent>,
    running: usize,
    binding: bool,
}

impl Jobs {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        let (sender, receiver) = channel();
        Self {
            sender,
            receiver,
            proxy,
            running: 0,
            binding: false,
        }
    }

    /// Nombre de tâches en cours.
    pub fn running(&self) -> usize {
        self.running
    }

    pub fn is_binding(&self) -> bool {
        self.binding
    }

    fn spawn(&mut self, work: impl FnOnce() -> JobResult + Send + 'static) {
        self.running += 1;
        let sender = self.sender.clone();
        let proxy = self.proxy.clone();
        std::thread::spawn(move || {
            let _ = sender.send(work());
            let _ = proxy.send_event(UserEvent::JobDone);
        });
    }

    pub fn bind(&mut self, job: BindJob) {
        self.binding = true;
        self.spawn(move || {
            let (skin, key) = job.run();
            JobResult::Bound(skin, key)
        });
    }

    pub fn export(&mut self, rig: Rig, format: Format, path: PathBuf) {
        self.spawn(move || {
            JobResult::Exported(bones_core::io::export(&rig, format, &path).map_err(|e| e.to_string()))
        });
    }

    /// Résultats terminés depuis le dernier appel.
    pub fn poll(&mut self) -> Vec<JobResult> {
        let results: Vec<JobResult> = self.receiver.try_iter().collect();
        self.running -= results.len().min(self.running);
        if results.iter().any(|r| matches!(r, JobResult::Bound(..))) {
            self.binding = false;
        }
        results
    }
}
