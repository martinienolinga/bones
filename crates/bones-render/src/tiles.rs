//! Vignettes 3D rendues chacune dans sa propre texture, sur fond
//! transparent (alpha prémultiplié) : l'interface les pose sur ses cartes
//! comme des images natives (galeries de préréglages).
//!
//! Une texture par vignette plutôt qu'un atlas : la passe de rendu efface et
//! résout (MSAA) toute sa cible, les vignettes ont des tailles différentes
//! (cartes, bandeau d'une feuille), et l'on évite une copie. Chaque vignette
//! est soumise aussitôt : ses uniformes (caméra, matrices des os) sont ainsi
//! lus avant que la suivante ne les remplace.

use crate::renderer::Renderer;
use crate::scene::Scene;
use crate::targets::RenderTargets;

/// Attachements gardés pour des tailles différentes (cartes, bandeau…).
const TARGET_SIZES: usize = 3;

struct Tile {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    /// Change à chaque (re)création de la texture.
    generation: u64,
}

/// Vignettes rendues, par emplacement.
#[derive(Default)]
pub struct Tiles {
    slots: Vec<Option<Tile>>,
    /// Attachements (profondeur, MSAA) par taille, du plus récent au plus ancien.
    targets: Vec<RenderTargets>,
    generation: u64,
}

impl Tiles {
    pub fn new() -> Self {
        Self::default()
    }

    /// Vue de la vignette `slot` (format du moteur) et sa génération : une
    /// nouvelle génération signale une nouvelle texture à réenregistrer.
    pub fn view(&self, slot: usize) -> Option<(&wgpu::TextureView, u64)> {
        let tile = self.slots.get(slot)?.as_ref()?;
        Some((&tile.view, tile.generation))
    }

    /// Texture de la vignette `slot` (relecture dans les tests).
    pub fn texture(&self, slot: usize) -> Option<&wgpu::Texture> {
        Some(&self.slots.get(slot)?.as_ref()?.texture)
    }

    /// Taille en pixels de la vignette `slot`.
    pub fn size(&self, slot: usize) -> Option<[u32; 2]> {
        self.texture(slot).map(|t| [t.width(), t.height()])
    }

    /// Nombre d'emplacements.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Ne garde que les `len` premiers emplacements (mémoire libérée).
    pub fn truncate(&mut self, len: usize) {
        self.slots.truncate(len);
        if self.slots.is_empty() {
            self.targets.clear();
        }
    }

    /// Prépare `scene` et la dessine dans la vignette `slot` de `size` pixels
    /// (créée ou recréée au besoin), sur fond transparent. Le fond de la
    /// scène est ignoré. La commande est soumise immédiatement.
    pub fn render(
        &mut self,
        renderer: &mut Renderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &Scene,
        slot: usize,
        size: [u32; 2],
    ) {
        let size = [size[0].max(1), size[1].max(1)];
        let format = renderer.targets.format();
        let view = self.ensure(device, format, slot, size).view.clone();
        renderer.prepare(device, queue, scene);
        // Fond noir transparent : les bords anticrénelés restent en alpha
        // prémultiplié, comme l'attend egui.
        renderer.frame.background = [0.0; 3];
        renderer.frame.clear_alpha = 0.0;
        let targets = self.targets(device, format, renderer.targets.samples(), size);
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("tile") });
        renderer.draw(&mut encoder, targets.views(), &view, size);
        queue.submit([encoder.finish()]);
    }

    /// Vignette `slot` à la bonne taille.
    fn ensure(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        slot: usize,
        size: [u32; 2],
    ) -> &Tile {
        if self.slots.len() <= slot {
            self.slots.resize_with(slot + 1, || None);
        }
        let fits = self.slots[slot]
            .as_ref()
            .is_some_and(|t| [t.texture.width(), t.texture.height()] == size && t.texture.format() == format);
        if !fits {
            self.generation += 1;
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("tile"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.slots[slot] = Some(Tile {
                texture,
                view,
                generation: self.generation,
            });
        }
        self.slots[slot].as_ref().expect("vignette créée")
    }

    /// Attachements pour `size`, réutilisés d'une image à l'autre.
    fn targets(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        samples: u32,
        size: [u32; 2],
    ) -> &RenderTargets {
        let found = self.targets.iter().position(|t| t.size() == size);
        let mut targets = match found {
            Some(i) => self.targets.remove(i),
            None => RenderTargets::new(format, samples),
        };
        targets.ensure(device, size);
        self.targets.insert(0, targets);
        self.targets.truncate(TARGET_SIZES);
        &self.targets[0]
    }
}
