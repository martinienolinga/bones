//! Tâches de fond : tout ce qui peut prendre plus d'une image (liaison des
//! poids, lecture/écriture de fichiers, import, export) tourne hors du fil
//! principal pour garder l'interface fluide. Chaque tâche réveille la boucle
//! d'événements quand elle se termine.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};

use bones_core::editor::BindJob;
use bones_core::{Mesh, Rig, Skin};
use winit::event_loop::EventLoopProxy;

use crate::UserEvent;

pub enum JobResult {
    Bound(Skin, (u64, u64)),
    Opened(Result<Rig, String>),
    Imported(Result<Mesh, String>),
    Saved(Result<PathBuf, String>),
    Exported(Result<PathBuf, String>),
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

    pub fn open(&mut self, path: PathBuf) {
        self.spawn(move || {
            let result = std::fs::File::open(&path)
                .map_err(|e| e.to_string())
                .and_then(|f| Rig::load(f).map_err(|e| e.to_string()));
            JobResult::Opened(result)
        });
    }

    pub fn import_obj(&mut self, path: PathBuf) {
        self.spawn(move || {
            let result = std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|src| bones_core::io::obj::parse(&src).map_err(|e| e.to_string()));
            JobResult::Imported(result)
        });
    }

    /// Enregistre une copie du document (clone peu coûteux grâce aux `Arc`).
    pub fn save(&mut self, rig: Rig, path: PathBuf) {
        self.spawn(move || {
            let result = write_atomically(&path, |f| rig.save(f).map_err(|e| e.to_string()));
            JobResult::Saved(result.map(|_| path))
        });
    }

    pub fn export_glb(&mut self, rig: Rig, path: PathBuf) {
        self.spawn(move || {
            let bytes = bones_core::io::gltf::export_glb(&rig);
            let result = write_atomically(&path, |mut f| {
                std::io::Write::write_all(&mut f, &bytes).map_err(|e| e.to_string())
            });
            JobResult::Exported(result.map(|_| path))
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
