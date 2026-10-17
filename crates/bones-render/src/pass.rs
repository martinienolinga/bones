//! Dessin d'une image en une seule passe : maillage texturé et grille, puis
//! surcouche « rayons X » (os, poignées, trajectoires).

use crate::renderer::Renderer;
use crate::targets::Views;

/// Partage de la plage de profondeur : la scène (grille, maillage) occupe
/// [SPLIT, 1] et la surcouche « rayons X » (os, poignées) [0, SPLIT]. La
/// surcouche passe ainsi toujours devant le maillage tout en gardant un test
/// de profondeur entre os, le tout dans une seule passe — ce qui permet des
/// attachements sans mémoire sur iPad.
const OVERLAY_DEPTH_SPLIT: f32 = 0.1;

impl Renderer {
    /// Dessine la scène préparée dans `target` (taille `size` en pixels).
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        size: [u32; 2],
    ) {
        self.targets.ensure(device, size);
        self.draw(encoder, self.targets.views(), target, size);
    }

    /// Passe de rendu de la scène préparée, avec les attachements `views`
    /// (profondeur, couleur multi-échantillonnée) de la taille de `target`.
    pub(crate) fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        views: &Views,
        target: &wgpu::TextureView,
        size: [u32; 2],
    ) {
        let f = &self.frame;
        let [x, y, w, h] = clamp_viewport(f.viewport, size);
        let [r, g, b] = f.background;
        let (view, resolve_target, store) = match &views.msaa {
            Some(msaa) => (msaa, Some(target), wgpu::StoreOp::Discard),
            None => (target, None, wgpu::StoreOp::Store),
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a: 1.0 }),
                    store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &views.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &self.bind_group, &[]);

        // Scène : maillage (une plage d'indices par matériau) puis grille.
        pass.set_viewport(x, y, w, h, OVERLAY_DEPTH_SPLIT, 1.0);
        if f.show_mesh
            && let Some(mesh) = &self.mesh
            && let Some(weights) = &mesh.weights
            && let (Some(materials), Some(ctx)) = (&self.materials, &self.material_context)
        {
            pass.set_pipeline(&self.pipelines.mesh);
            pass.set_vertex_buffer(0, mesh.positions.slice(..));
            pass.set_vertex_buffer(1, mesh.normals.slice(..));
            pass.set_vertex_buffer(2, weights.slice(..));
            pass.set_vertex_buffer(3, mesh.uv_source().slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            for (range, material) in &materials.parts {
                pass.set_bind_group(1, materials.group(ctx, *material), &[]);
                pass.draw_indexed(range.clone(), 0, 0..1);
            }
        }
        if f.lines > 0 {
            pass.set_pipeline(&self.pipelines.line);
            pass.set_vertex_buffer(0, self.lines.buffer.slice(..));
            pass.draw(0..f.lines, 0..1);
        }

        // Surcouche « rayons X » : os, poignées, trajectoires.
        pass.set_viewport(x, y, w, h, 0.0, OVERLAY_DEPTH_SPLIT);
        if f.bones > 0 {
            pass.set_pipeline(&self.pipelines.bone);
            pass.set_vertex_buffer(0, self.octahedron.slice(..));
            pass.set_vertex_buffer(1, self.instances.buffer.slice(..));
            pass.draw(0..self.octahedron_vertices, 0..f.bones);
        }
        if f.overlay_lines > 0 {
            pass.set_pipeline(&self.pipelines.overlay_line);
            pass.set_vertex_buffer(0, self.overlay_lines.buffer.slice(..));
            pass.draw(0..f.overlay_lines, 0..1);
        }
    }
}

/// Restreint la zone de rendu à la cible (wgpu refuse un viewport qui déborde).
fn clamp_viewport(viewport: [f32; 4], size: [u32; 2]) -> [f32; 4] {
    let [x, y, w, h] = viewport;
    let (sw, sh) = (size[0] as f32, size[1] as f32);
    let (x, y) = (x.clamp(0.0, sw - 1.0), y.clamp(0.0, sh - 1.0));
    [x, y, w.min(sw - x).max(1.0), h.min(sh - y).max(1.0)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_is_clamped_to_target() {
        assert_eq!(
            clamp_viewport([0.0, 0.0, 800.0, 600.0], [800, 600]),
            [0.0, 0.0, 800.0, 600.0]
        );
        assert_eq!(
            clamp_viewport([-10.0, 50.0, 900.0, 600.0], [800, 600]),
            [0.0, 50.0, 800.0, 550.0]
        );
        assert_eq!(
            clamp_viewport([0.0, 0.0, 0.0, 0.0], [800, 600]),
            [0.0, 0.0, 1.0, 1.0]
        );
    }
}
