//! Vignettes 3D des galeries de préréglages : le mannequin de l'app joue
//! chaque préréglage, rendu sur le GPU directement dans une texture native
//! d'egui (aucune relecture des pixels).
//!
//! Déroulement d'une image, comme pour le verre dépoli :
//! 1. [`Showcase::publish`] avant l'interface : textures egui disponibles,
//!    par emplacement ;
//! 2. l'interface demande une vignette par carte visible ([`request`]), avec
//!    les matrices des os et la caméra ;
//! 3. [`Showcase::render`] après l'interface, avant la passe d'egui : les
//!    vignettes dont le contenu a changé sont rendues dans leurs textures
//!    (une expression n'est rendue qu'une fois, une animation à chaque
//!    nouvelle image de son cycle).
//!
//! Le moteur de rendu des vignettes est distinct de celui de la vue 3D (il
//! garde le maillage du mannequin, sans toucher à celui du projet) ; il est
//! créé à la première vignette. Les textures sont libérées quand plus
//! aucune vignette n'est demandée pendant quelques secondes, le moteur
//! lui-même lors d'une alerte mémoire.

mod requests;

use std::time::{Duration, Instant};

use bones_render::{Renderer, Scene, Tiles};

pub use requests::{TileRequest, request};

/// Délai sans demande après lequel les textures des vignettes sont libérées.
const IDLE: Duration = Duration::from_secs(3);

pub struct Showcase {
    format: wgpu::TextureFormat,
    renderer: Option<Renderer>,
    tiles: Tiles,
    /// Texture egui et génération de la texture GPU, par emplacement.
    textures: Vec<Option<(egui::TextureId, u64)>>,
    /// Clé du contenu rendu, par emplacement.
    keys: Vec<Option<u64>>,
    last_request: Option<Instant>,
}

impl Showcase {
    pub fn new(format: wgpu::TextureFormat) -> Self {
        Self {
            format,
            renderer: None,
            tiles: Tiles::new(),
            textures: Vec::new(),
            keys: Vec::new(),
            last_request: None,
        }
    }

    /// Avant l'interface : textures disponibles.
    pub fn publish(&self, ctx: &egui::Context) {
        requests::publish(ctx, self.textures.iter().map(|t| t.map(|(id, _)| id)).collect());
    }

    /// Après l'interface, avant la passe d'egui : rend les vignettes
    /// demandées à cette image.
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        egui_renderer: &mut egui_wgpu::Renderer,
    ) {
        let batch = requests::take(ctx);
        let (Some(model), false) = (batch.model, batch.requests.is_empty()) else {
            if self.last_request.is_some_and(|t| t.elapsed() > IDLE) {
                self.release_tiles(egui_renderer);
            }
            return;
        };
        self.last_request = Some(Instant::now());
        let renderer = self
            .renderer
            .get_or_insert_with(|| Renderer::new(device, self.format));
        let revisions = model.revisions();
        renderer.set_mesh(device, queue, &model.mesh, revisions.mesh);
        renderer.set_skin(device, queue, &model.skin, revisions.skin);
        let count = batch.requests.len();
        self.keys.resize(self.keys.len().max(count), None);
        self.textures.resize(self.textures.len().max(count), None);
        for (slot, tile) in batch.requests.iter().enumerate() {
            let fresh = self.keys[slot] == Some(tile.key) && self.tiles.size(slot) == Some(tile.size);
            if fresh {
                continue;
            }
            let [w, h] = tile.size;
            let scene = Scene {
                view_proj: tile.view_proj,
                eye: tile.eye,
                viewport: [0.0, 0.0, w as f32, h as f32],
                background: [0.0; 3],
                joints: &tile.joints,
                show_mesh: true,
                textured: true,
                weights: None,
                lines: &[],
                bones: &[],
                overlay_lines: &[],
            };
            self.tiles
                .render(renderer, device, queue, &scene, slot, tile.size);
            self.keys[slot] = Some(tile.key);
            if let Some((view, generation)) = self.tiles.view(slot) {
                register(&mut self.textures[slot], view, generation, device, egui_renderer);
            }
        }
    }

    /// Libère les textures des vignettes (galerie refermée). Le moteur de
    /// rendu et le maillage du mannequin restent prêts pour la réouverture.
    fn release_tiles(&mut self, egui_renderer: &mut egui_wgpu::Renderer) {
        for (id, _) in self.textures.drain(..).flatten() {
            egui_renderer.free_texture(&id);
        }
        self.keys.clear();
        self.tiles.truncate(0);
        self.last_request = None;
    }

    /// Alerte mémoire : tout est libéré, moteur de rendu compris.
    pub fn release(&mut self, egui_renderer: &mut egui_wgpu::Renderer) {
        self.release_tiles(egui_renderer);
        self.renderer = None;
    }
}

/// Enregistre (ou met à jour, si la texture GPU a été recréée) la texture
/// egui d'un emplacement.
fn register(
    texture: &mut Option<(egui::TextureId, u64)>,
    view: &wgpu::TextureView,
    generation: u64,
    device: &wgpu::Device,
    egui_renderer: &mut egui_wgpu::Renderer,
) {
    let filter = wgpu::FilterMode::Linear;
    *texture = Some(match *texture {
        Some((id, old)) if old == generation => (id, old),
        Some((id, _)) => {
            egui_renderer.update_egui_texture_from_wgpu_texture(device, view, filter, id);
            (id, generation)
        }
        None => (
            egui_renderer.register_native_texture(device, view, filter),
            generation,
        ),
    });
}

/// Clés des vignettes demandées à cette image (vidées), pour les tests
/// d'interface sans GPU.
#[cfg(test)]
pub fn take_keys(ctx: &egui::Context) -> Vec<u64> {
    requests::take(ctx).requests.iter().map(|r| r.key).collect()
}
