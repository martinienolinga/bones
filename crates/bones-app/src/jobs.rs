//! Tâches de fond : tout ce qui peut prendre plus d'une image (liaison des
//! poids, lecture/écriture de fichiers, import, export) tourne hors du fil
//! principal pour garder l'interface fluide. Chaque tâche réveille la boucle
//! d'événements quand elle se termine.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};

use bones_core::editor::BindJob;
use bones_core::io::{Format, Imported};
use bones_core::{Rig, Skin};
use winit::event_loop::EventLoopProxy;

use crate::UserEvent;

pub enum JobResult {
    Bound(Skin, (u64, u64)),
    Imported(Result<Imported, String>),
    Saved(Result<PathBuf, String>),
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

    /// Ouvre un document `.bones` ou importe un fichier 3D (format déduit
    /// de l'extension, textures cherchées à côté du fichier).
    pub fn import(&mut self, path: PathBuf) {
        self.spawn(move || JobResult::Imported(bones_core::io::import(&path).map_err(|e| e.to_string())));
    }

    /// Enregistre une copie du document (clone peu coûteux grâce aux `Arc`).
    pub fn save(&mut self, rig: Rig, path: PathBuf) {
        self.spawn(move || {
            let result = write_atomically(&path, |f| rig.save(f).map_err(|e| e.to_string()));
            JobResult::Saved(result.map(|_| path))
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

/// Écrit dans un fichier temporaire puis le renomme : un document n'est
/// jamais laissé à moitié écrit.
fn write_atomically(
    path: &Path,
    write: impl FnOnce(std::fs::File) -> Result<(), String>,
) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    let file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    write(file)?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}
