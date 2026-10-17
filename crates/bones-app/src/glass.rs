//! Verre dépoli des panneaux : relie l'interface (`bones_ui::glass`), le flou
//! GPU (`bones_render::Backdrop`) et les textures natives d'egui.
//!
//! À chaque image : [`Glass::publish`] avant l'interface, puis
//! [`Glass::render`] après la scène 3D et avant la passe d'egui. Si la
//! surface ne permet pas la copie (`COPY_SRC`), rien n'est flouté et les
//! panneaux gardent leur fond sombre uni.

use bones_render::{Backdrop, Region};

pub struct Glass {
    backdrop: Option<Backdrop>,
    /// Texture egui et génération de la sortie floutée, par emplacement.
    textures: Vec<Option<(egui::TextureId, u64)>>,
}

impl Glass {
    /// `copyable` : la surface accepte `COPY_SRC`.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, copyable: bool) -> Self {
        Self {
            backdrop: copyable.then(|| Backdrop::new(device, format)),
            textures: Vec::new(),
        }
    }

    /// Avant l'interface : textures floutées disponibles.
    pub fn publish(&self, ctx: &egui::Context) {
        let ids = self.textures.iter().map(|t| t.map(|(id, _)| id)).collect();
        bones_ui::glass::publish(ctx, ids);
    }

    /// Après la scène 3D (dans `source`), avant la passe d'egui : floute les
    /// zones demandées par l'interface et met à jour les textures d'egui.
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        egui_renderer: &mut egui_wgpu::Renderer,
        source: &wgpu::Texture,
    ) {
        let requests = bones_ui::glass::take_requests(ctx);
        let Some(backdrop) = &mut self.backdrop else {
            return;
        };
        let (screen, ppp) = (ctx.content_rect(), ctx.pixels_per_point());
        let regions: Vec<Region> = requests
            .iter()
            .map(|rect| {
                let r = bones_ui::glass::region(*rect, screen);
                let (x0, y0) = ((r.min.x * ppp).floor().max(0.0), (r.min.y * ppp).floor().max(0.0));
                let (x1, y1) = ((r.max.x * ppp).ceil(), (r.max.y * ppp).ceil());
                Region {
                    x: x0 as u32,
                    y: y0 as u32,
                    width: (x1 - x0).max(0.0) as u32,
                    height: (y1 - y0).max(0.0) as u32,
                }
            })
            .collect();
        if regions.is_empty() {
            return;
        }
        backdrop.blur(device, encoder, source, &regions);
        if self.textures.len() < regions.len() {
            self.textures.resize(regions.len(), None);
        }
        let filter = wgpu::FilterMode::Linear;
        for (slot, texture) in self.textures.iter_mut().enumerate().take(regions.len()) {
            *texture = match (backdrop.output(slot), *texture) {
                (Some((view, generation)), None) => Some((
                    egui_renderer.register_native_texture(device, view, filter),
                    generation,
                )),
                (Some((view, generation)), Some((id, old))) if generation != old => {
                    egui_renderer.update_egui_texture_from_wgpu_texture(device, view, filter, id);
                    Some((id, generation))
                }
                (None, Some((id, _))) => {
                    egui_renderer.free_texture(&id);
                    None
                }
                (_, current) => current,
            };
        }
    }
}
