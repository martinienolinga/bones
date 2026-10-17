//! Travaux disque sur les projets (exécutés par le fil de [`super::worker`])
//! et leurs résultats.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bones_core::glam::Vec3;
use bones_core::project::{self, LoadedProject, ProjectInfo, ProjectSnapshot, store};
use bones_core::{Editor, Rig, head_tail};

use crate::files::{self, ImportFile};

/// Image RGBA 8 bits (vignette relue sur le GPU).
pub struct Rgba {
    pub size: [u32; 2],
    pub pixels: Vec<u8>,
}

/// Production de la vignette, appelée sur le fil des projets (elle attend
/// la fin du rendu GPU).
pub type ThumbnailSource = Box<dyn FnOnce() -> Option<Rgba> + Send>;

/// Construction du document d'un nouveau projet (modèle, import 3D).
pub type Build = Box<dyn FnOnce() -> Result<Rig, String> + Send>;

pub struct SaveTask {
    pub path: PathBuf,
    pub name: String,
    pub created: u64,
    pub snapshot: ProjectSnapshot,
    pub thumbnail: Option<ThumbnailSource>,
}

pub enum Task {
    Save(Box<SaveTask>),
    Load(PathBuf),
    /// Nouveau projet nommé d'après `name` (rendu unique), enregistré tout
    /// de suite ; `open` : à ouvrir dans l'éditeur.
    Create {
        name: String,
        build: Build,
        open: bool,
    },
    Rename(PathBuf, String),
    Duplicate(PathBuf),
    Delete(PathBuf),
    /// Projet reçu (dossier Documents ou boîte de réception) à ranger dans
    /// le dossier des projets.
    Adopt(PathBuf),
    /// Liste des projets (galerie).
    List,
    /// Fichiers importables du dossier Documents.
    ListImports,
}

/// Projet créé et déjà enregistré (sans vignette).
pub struct Created {
    pub path: PathBuf,
    pub info: ProjectInfo,
    pub editor: Box<Editor>,
}

pub enum Event {
    /// Vignette fraîche, envoyée avant la fin de l'enregistrement.
    Thumbnail {
        path: PathBuf,
        image: egui::ColorImage,
    },
    Saved {
        path: PathBuf,
        result: Result<(ProjectInfo, SystemTime), String>,
    },
    Loaded {
        path: PathBuf,
        result: Result<Box<LoadedProject>, String>,
        /// Vignette décodée en pleine résolution (transition d'ouverture).
        preview: Option<egui::ColorImage>,
    },
    Created {
        open: bool,
        result: Result<Created, String>,
    },
    Renamed {
        from: PathBuf,
        result: Result<PathBuf, String>,
    },
    Duplicated(Result<PathBuf, String>),
    Deleted {
        path: PathBuf,
        result: Result<(), String>,
    },
    Adopted(Result<PathBuf, String>),
    Listed(Vec<store::Entry>),
    Imports(Vec<ImportFile>),
    /// Travail interrompu par une erreur interne (panique).
    Failed(String),
}

/// Dossiers de travail.
#[derive(Clone, Debug)]
pub struct Dirs {
    /// `Documents/Projects`.
    pub projects: PathBuf,
    /// `Documents` (fichiers à importer).
    pub documents: PathBuf,
}

/// Exécute `task` ; `send` reçoit les résultats (parfois deux : vignette
/// puis enregistrement).
pub fn run(task: Task, dirs: &Dirs, send: &mut dyn FnMut(Event)) {
    let text = |e: project::ProjectError| e.to_string();
    match task {
        Task::Save(task) => save(*task, dirs, send),
        Task::Load(path) => {
            let result = project::load(&path).map(Box::new).map_err(text);
            let preview = result
                .as_ref()
                .ok()
                .and_then(|p| p.thumbnail.as_deref())
                .and_then(|jpeg| project::decode_thumbnail(jpeg).ok())
                .map(|(w, h, rgba)| {
                    egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba)
                });
            send(Event::Loaded {
                path,
                result,
                preview,
            });
        }
        Task::Create { name, build, open } => send(Event::Created {
            open,
            result: create(&name, build, dirs),
        }),
        Task::Rename(path, name) => send(Event::Renamed {
            result: store::rename(&path, &name).map_err(text),
            from: path,
        }),
        Task::Duplicate(path) => send(Event::Duplicated(store::duplicate(&path).map_err(text))),
        Task::Delete(path) => send(Event::Deleted {
            result: store::delete(&path).map_err(text),
            path,
        }),
        Task::Adopt(path) => send(Event::Adopted(adopt(&path, dirs))),
        Task::List => send(Event::Listed(store::list(&dirs.projects))),
        Task::ListImports => send(Event::Imports(files::importable(&dirs.documents))),
    }
}

fn save(task: SaveTask, dirs: &Dirs, send: &mut dyn FnMut(Event)) {
    let SaveTask {
        path,
        name,
        created,
        snapshot,
        thumbnail,
    } = task;
    let mut jpeg = None;
    if let Some(Rgba { size, pixels }) = thumbnail.and_then(|f| f()) {
        let image = egui::ColorImage::from_rgba_unmultiplied([size[0] as usize, size[1] as usize], &pixels);
        send(Event::Thumbnail {
            path: path.clone(),
            image,
        });
        match project::encode_thumbnail(size[0], size[1], &pixels) {
            Ok(bytes) => jpeg = Some(bytes),
            Err(e) => log::warn!("vignette non encodée : {e}"),
        }
    }
    let _ = std::fs::create_dir_all(&dirs.projects);
    let result = snapshot
        .save(&path, &name, created, jpeg.as_deref())
        .map_err(|e| e.to_string())
        .map(|info| (info, modified(&path)));
    send(Event::Saved { path, result });
}

fn create(name: &str, build: Build, dirs: &Dirs) -> Result<Created, String> {
    let rig = build()?;
    std::fs::create_dir_all(&dirs.projects).map_err(|e| e.to_string())?;
    let path = store::unique_path(&dirs.projects, name);
    let name = files::stem(&path);
    let mut editor = Editor::new(rig);
    editor.rig.name = name.clone();
    if editor.rig.mesh.triangle_count() == 0 {
        frame_skeleton(&mut editor);
    }
    let info = ProjectSnapshot::capture(&editor)
        .save(&path, &name, project::unix_now(), None)
        .map_err(|e| e.to_string())?;
    Ok(Created {
        path,
        info,
        editor: Box::new(editor),
    })
}

/// Sans maillage (projet vide, BVH), la vue cadre le squelette, dans une
/// boîte d'au moins 0,8 × 1,5 m pour qu'un os seul ne remplisse pas l'écran.
fn frame_skeleton(editor: &mut Editor) {
    let world = editor.rig.world();
    let points = world.iter().zip(editor.rig.skeleton.bones()).flat_map(|(w, b)| {
        let (head, tail) = head_tail(w, b.length);
        [head, tail]
    });
    let (lo, hi) = points.fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), p| {
        (lo.min(p), hi.max(p))
    });
    if lo.x > hi.x {
        return;
    }
    let pad = ((Vec3::new(0.8, 1.5, 0.8) - (hi - lo)) * 0.5).max(Vec3::ZERO);
    editor.camera.frame(lo - pad, hi + pad);
}

/// Range un projet reçu : copié depuis Documents, déplacé depuis la boîte
/// de réception (iOS demande de la vider). Le fichier est vérifié avant.
fn adopt(source: &Path, dirs: &Dirs) -> Result<PathBuf, String> {
    project::read_info(source).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dirs.projects).map_err(|e| e.to_string())?;
    let target = store::unique_path(&dirs.projects, &files::stem(source));
    let from_inbox = source.parent().is_some_and(|p| p.ends_with(files::INBOX));
    let moved = from_inbox && std::fs::rename(source, &target).is_ok();
    if !moved {
        std::fs::copy(source, &target).map_err(|e| e.to_string())?;
    }
    Ok(target)
}

fn modified(path: &Path) -> SystemTime {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or_else(|_| SystemTime::now())
}
