//! Vignettes des cartes rendues avec le mannequin de l'app
//! (`samples::mannequin`) : animations humanoïdes jouées sur place, corps
//! entier ; expressions en gros plan sur le visage.
//!
//! Un fil de fond charge le mannequin une fois, puis cuit chaque préréglage
//! demandé ([`Staged`]) : le fil de l'interface ne fait que relever les
//! résultats, calculer les matrices des os de l'image voulue (quelques
//! microsecondes) et demander la vignette au rendu (`crate::showcase`). Les
//! matrices d'une expression (pose fixe) sont calculées une seule fois.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use bones_core::presets::{Preset, StageModel, Staged, shot_for};
use bones_core::{Rig, Transform, Viewport};
use glam::Mat4;

use crate::showcase::{self, TileRequest};

/// Vignette d'une carte.
pub enum Tile {
    /// Texture prête à poser sur la carte.
    Texture(egui::TextureId),
    /// En préparation : la carte garde son repère.
    Pending,
    /// Préréglage sans sens sur un humain (animal, végétal…).
    Unsupported,
}

/// Résultat du fil de fond.
enum Done {
    Model(Arc<Rig>, Arc<StageModel>),
    Staged(String, Option<Staged>),
}

/// Préréglage cuit, et ses matrices s'il est fixe.
struct Ready {
    staged: Staged,
    still: Option<Arc<[Mat4]>>,
}

#[derive(Default)]
pub struct StageCache {
    jobs: Option<Sender<Preset>>,
    results: Option<Receiver<Done>>,
    model: Option<(Arc<Rig>, Arc<StageModel>)>,
    /// Par identifiant : `None` tant que la cuisson est en cours ou a échoué.
    staged: HashMap<String, Option<Ready>>,
    /// Cuissons demandées et pas encore revenues.
    pending: usize,
    world: Vec<Transform>,
    joints: Vec<Mat4>,
}

impl StageCache {
    /// Oublie les préréglages cuits (bibliothèque relue) ; le mannequin reste.
    pub fn clear(&mut self) {
        self.staged.clear();
    }

    /// Des cuissons sont en cours (tests).
    #[cfg(test)]
    pub fn busy(&self) -> bool {
        self.pending > 0 || (self.jobs.is_some() && self.model.is_none())
    }

    /// Le préréglage est-il joué en boucle (vignette à redessiner) ?
    pub fn animated(&self, preset: &Preset) -> bool {
        self.ready(preset).is_some_and(|r| !r.staged.is_static())
    }

    fn ready(&self, preset: &Preset) -> Option<&Ready> {
        self.staged.get(&preset.info.id)?.as_ref()
    }

    /// Vignette de `preset` dans une carte de `size` points, à l'instant
    /// `time` (secondes), avec une marge relative `margin` autour du cadrage.
    pub fn tile(
        &mut self,
        ctx: &egui::Context,
        preset: &Preset,
        size: egui::Vec2,
        time: f32,
        margin: f32,
    ) -> Tile {
        if shot_for(preset).is_none() {
            return Tile::Unsupported;
        }
        self.poll();
        let id = &preset.info.id;
        if !self.staged.contains_key(id) {
            self.submit(preset);
            return Tile::Pending;
        }
        let (Some((rig, model)), Some(Some(ready))) = (&self.model, self.staged.get(id)) else {
            return Tile::Pending;
        };
        let ppp = ctx.pixels_per_point();
        let px = [
            (size.x * ppp).round().max(1.0) as u32,
            (size.y * ppp).round().max(1.0) as u32,
        ];
        let viewport = Viewport::new(px[0] as f32, px[1] as f32);
        let camera = ready.staged.camera(viewport.aspect(), margin);
        let frame = ready.staged.frame_at(time);
        let joints = match &ready.still {
            Some(j) => j.clone(),
            None => {
                ready
                    .staged
                    .joints(model, frame, &mut self.world, &mut self.joints);
                Arc::from(self.joints.as_slice())
            }
        };
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (id, frame, px, margin.to_bits()).hash(&mut hasher);
        let request = TileRequest {
            key: hasher.finish(),
            size: px,
            joints,
            view_proj: camera.view_proj(&viewport),
            eye: camera.eye(),
        };
        match showcase::request(ctx, rig, request) {
            Some(texture) => Tile::Texture(texture),
            None => Tile::Pending,
        }
    }

    /// Envoie `preset` au fil de fond (lancé à la première demande).
    fn submit(&mut self, preset: &Preset) {
        let jobs = self.jobs.get_or_insert_with(|| {
            let (jobs, results) = spawn();
            self.results = Some(results);
            jobs
        });
        if jobs.send(preset.clone()).is_ok() {
            self.staged.insert(preset.info.id.clone(), None);
            self.pending += 1;
        }
    }

    /// Relève les résultats du fil de fond.
    fn poll(&mut self) {
        let Some(results) = &self.results else { return };
        for done in results.try_iter() {
            match done {
                Done::Model(rig, model) => self.model = Some((rig, model)),
                Done::Staged(id, staged) => {
                    self.pending = self.pending.saturating_sub(1);
                    let ready = staged.map(|staged| {
                        let still = self.model.as_ref().filter(|_| staged.is_static()).map(|(_, m)| {
                            staged.joints(m, 0, &mut self.world, &mut self.joints);
                            Arc::from(self.joints.as_slice())
                        });
                        Ready { staged, still }
                    });
                    self.staged.insert(id, ready);
                }
            }
        }
    }
}

/// Fil de fond : charge le mannequin, puis cuit les préréglages reçus.
fn spawn() -> (Sender<Preset>, Receiver<Done>) {
    let (jobs, inbox) = channel::<Preset>();
    let (outbox, results) = channel();
    std::thread::Builder::new()
        .name("preset-stage".into())
        .spawn(move || {
            let rig = Arc::new(bones_core::samples::mannequin());
            let model = Arc::new(StageModel::new(&rig));
            if outbox.send(Done::Model(rig, model.clone())).is_err() {
                return;
            }
            for preset in inbox {
                let staged = Staged::new(&preset, &model).ok();
                if outbox.send(Done::Staged(preset.info.id.clone(), staged)).is_err() {
                    return;
                }
            }
        })
        .expect("fil des vignettes");
    (jobs, results)
}
