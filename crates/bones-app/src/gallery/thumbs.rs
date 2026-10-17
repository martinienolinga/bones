//! Vignettes de la galerie : lues et décodées hors du fil principal (deux
//! fils dédiés, la demande la plus récente d'abord), réduites à leur taille
//! d'affichage, puis envoyées à egui comme textures. Le cache est indexé par
//! fichier et date de modification ; seules les [`MAX_TEXTURES`] vignettes
//! les plus récemment affichées restent en mémoire.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};
use std::time::SystemTime;

use bones_core::project::{self, ProjectInfo};
use egui::{ColorImage, TextureHandle, TextureOptions};

use super::resize;
use crate::projects::Waker;

/// Textures gardées au plus (≈ 430 Ko chacune sur un iPad Retina).
pub const MAX_TEXTURES: usize = 40;
const THREADS: usize = 2;

/// Résultat d'une lecture : résumé et vignette décodée.
pub struct Preview {
    pub path: PathBuf,
    pub modified: SystemTime,
    pub info: Option<ProjectInfo>,
    pub image: Option<ColorImage>,
}

struct Request {
    path: PathBuf,
    modified: SystemTime,
    width: u32,
}

#[derive(Default)]
struct Slot {
    /// Version affichée ; `None` : vignette fraîche venue d'un
    /// enregistrement en cours (à jour quoi qu'il arrive).
    modified: Option<SystemTime>,
    texture: Option<TextureHandle>,
    loading: bool,
    /// Image utilisée à cette image d'interface (ordre d'éviction).
    used: u64,
}

/// Demandes en attente ; `quit` arrête les fils.
#[derive(Default)]
struct Pending {
    requests: Vec<Request>,
    quit: bool,
}

type Queue = Arc<(Mutex<Pending>, Condvar)>;

pub struct Thumbs {
    slots: HashMap<PathBuf, Slot>,
    queue: Queue,
    results: Receiver<Preview>,
    frame: u64,
}

impl Thumbs {
    pub fn new(wake: Waker) -> Self {
        let queue: Queue = Arc::default();
        let (sender, results) = channel();
        for i in 0..THREADS {
            let (queue, sender, wake) = (queue.clone(), sender.clone(), wake.clone());
            std::thread::Builder::new()
                .name(format!("thumbnails-{i}"))
                .spawn(move || serve(&queue, &sender, &wake))
                .expect("création d'un fil des vignettes");
        }
        Self {
            slots: HashMap::new(),
            queue,
            results,
            frame: 0,
        }
    }

    /// Vignette de la version `modified` du projet, demandée au besoin
    /// (largeur d'affichage `width` en pixels). L'ancienne image reste
    /// affichée pendant la lecture d'une nouvelle version.
    pub fn get(&mut self, path: &Path, modified: SystemTime, width: u32) -> Option<&TextureHandle> {
        let slot = self.slots.entry(path.to_owned()).or_default();
        slot.used = self.frame;
        let stale = slot.texture.is_none() || slot.modified.is_some_and(|m| m != modified);
        if stale && !slot.loading {
            slot.loading = true;
            let (lock, ready) = &*self.queue;
            let mut pending = lock.lock().unwrap_or_else(|e| e.into_inner());
            pending.requests.push(Request {
                path: path.to_owned(),
                modified,
                width,
            });
            ready.notify_one();
        }
        slot.texture.as_ref()
    }

    /// Lectures terminées : textures installées, résumés retournés.
    pub fn poll(&mut self, ctx: &egui::Context) -> Vec<Preview> {
        self.frame += 1;
        let mut previews: Vec<Preview> = self.results.try_iter().collect();
        for preview in &mut previews {
            let slot = self.slots.entry(preview.path.clone()).or_default();
            slot.loading = false;
            if slot.modified.is_none() && slot.texture.is_some() {
                continue; // vignette fraîche, plus récente que le fichier lu
            }
            slot.modified = Some(preview.modified);
            if let Some(image) = preview.image.take() {
                let name = format!("thumb:{}", preview.path.display());
                slot.texture = Some(ctx.load_texture(name, image, TextureOptions::LINEAR));
            }
        }
        self.evict();
        previews
    }

    /// Vignette fraîche du projet qu'on vient de quitter (avant la fin de
    /// son enregistrement).
    pub fn install(&mut self, ctx: &egui::Context, path: &Path, image: ColorImage) {
        let name = format!("thumb:{}", path.display());
        let slot = self.slots.entry(path.to_owned()).or_default();
        slot.texture = Some(ctx.load_texture(name, image, TextureOptions::LINEAR));
        slot.modified = None;
        slot.used = self.frame;
    }

    /// Enregistrement terminé : la vignette fraîche correspond désormais à
    /// la version `modified` du fichier.
    pub fn confirm(&mut self, path: &Path, modified: SystemTime) {
        if let Some(slot) = self.slots.get_mut(path).filter(|s| s.modified.is_none()) {
            slot.modified = Some(modified);
        }
    }

    pub fn texture(&self, path: &Path) -> Option<&TextureHandle> {
        self.slots.get(path)?.texture.as_ref()
    }

    pub fn moved(&mut self, from: &Path, to: &Path) {
        if let Some(slot) = self.slots.remove(from) {
            self.slots.insert(to.to_owned(), slot);
        }
    }

    pub fn forget(&mut self, path: &Path) {
        self.slots.remove(path);
    }

    /// Textures en mémoire.
    #[cfg(test)]
    pub fn textures(&self) -> usize {
        self.slots.values().filter(|s| s.texture.is_some()).count()
    }

    /// Libère les textures les moins récemment affichées au-delà de la limite.
    fn evict(&mut self) {
        let mut loaded: Vec<(u64, PathBuf)> = self
            .slots
            .iter()
            .filter(|(_, s)| s.texture.is_some())
            .map(|(p, s)| (s.used, p.clone()))
            .collect();
        if loaded.len() <= MAX_TEXTURES {
            return;
        }
        loaded.sort();
        let excess = loaded.len() - MAX_TEXTURES;
        for (used, path) in loaded.into_iter().take(excess) {
            if used + 1 >= self.frame {
                break; // encore à l'écran
            }
            self.slots.remove(&path);
        }
    }
}

impl Drop for Thumbs {
    fn drop(&mut self) {
        let (lock, ready) = &*self.queue;
        lock.lock().unwrap_or_else(|e| e.into_inner()).quit = true;
        ready.notify_all();
    }
}

fn serve(queue: &Queue, sender: &Sender<Preview>, wake: &Waker) {
    loop {
        let request = {
            let (lock, ready) = &**queue;
            let mut pending = lock.lock().unwrap_or_else(|e| e.into_inner());
            loop {
                if pending.quit {
                    return;
                }
                if let Some(request) = pending.requests.pop() {
                    break request;
                }
                pending = ready.wait(pending).unwrap_or_else(|e| e.into_inner());
            }
        };
        if sender.send(load(request)).is_err() {
            return;
        }
        wake();
    }
}

fn load(request: Request) -> Preview {
    let Request {
        path,
        modified,
        width,
    } = request;
    let (info, jpeg) = match project::read_info(&path) {
        Ok((info, jpeg)) => (Some(info), jpeg),
        Err(e) => {
            log::warn!("{} illisible : {e}", path.display());
            (None, None)
        }
    };
    let image = jpeg
        .and_then(|bytes| project::decode_thumbnail(&bytes).ok())
        .map(|(w, h, rgba)| {
            let (w, h, rgba) = resize::to_width(w, h, rgba, width);
            ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba)
        });
    Preview {
        path,
        modified,
        info,
        image,
    }
}
