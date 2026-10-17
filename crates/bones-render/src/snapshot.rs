//! Rendu hors écran et relecture des pixels : miniatures des projets (et
//! tests de rendu).
//!
//! L'image est dessinée avec les mêmes pipelines que l'écran (MSAA compris,
//! résolu dans une texture lisible), puis copiée dans un tampon. La commande
//! est soumise tout de suite et la lecture ([`Readback::wait`]) peut se faire
//! depuis un fil de fond : le fil principal ne bloque jamais sur le GPU.
//! Les attachements et le tampon n'existent que le temps d'une miniature.

use std::sync::mpsc;
use std::time::Duration;

use crate::renderer::Renderer;
use crate::scene::Scene;
use crate::targets::RenderTargets;

/// Attente maximale du GPU lors d'une relecture.
const TIMEOUT: Duration = Duration::from_secs(10);

/// Pixels d'une image rendue, en cours de copie vers un tampon lisible.
pub struct Readback {
    device: wgpu::Device,
    buffer: wgpu::Buffer,
    size: [u32; 2],
    /// Octets par ligne dans le tampon (alignés sur 256, imposé par wgpu).
    row_bytes: u32,
    /// Canaux dans l'ordre BGRA (format d'écran courant) plutôt que RGBA.
    bgra: bool,
}

/// Ordre des canaux d'un format 8 bits lisible : `Some(true)` pour BGRA,
/// `Some(false)` pour RGBA, `None` pour les autres formats.
fn channel_order(format: wgpu::TextureFormat) -> Option<bool> {
    use wgpu::TextureFormat as F;
    match format {
        F::Rgba8Unorm | F::Rgba8UnormSrgb => Some(false),
        F::Bgra8Unorm | F::Bgra8UnormSrgb => Some(true),
        _ => None,
    }
}

impl Readback {
    /// Encode dans `encoder` la copie de `texture` (créée avec
    /// `COPY_SRC`) vers un tampon lisible. L'appelant soumet `encoder`
    /// avant [`Self::wait`]. `None` si le format n'est pas un RGBA ou BGRA
    /// 8 bits.
    pub fn copy(
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
    ) -> Option<Self> {
        let bgra = channel_order(texture.format())?;
        let size = [texture.width(), texture.height()];
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let row_bytes = (size[0] * 4).div_ceil(align) * align;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(row_bytes) * u64::from(size[1]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row_bytes),
                    rows_per_image: None,
                },
            },
            texture.size(),
        );
        Some(Self {
            device: device.clone(),
            buffer,
            size,
            row_bytes,
            bgra,
        })
    }

    /// Largeur et hauteur de l'image, en pixels.
    pub fn size(&self) -> [u32; 2] {
        self.size
    }

    /// Attend la fin du rendu et retourne les pixels RGBA 8 bits, lignes
    /// contiguës de haut en bas. Bloquant : à appeler depuis un fil de fond
    /// (ou lors du passage en arrière-plan). `None` si le GPU a échoué.
    pub fn wait(self) -> Option<Vec<u8>> {
        let (sender, receiver) = mpsc::channel();
        self.buffer.map_async(wgpu::MapMode::Read, .., move |result| {
            let _ = sender.send(result);
        });
        let poll = wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(TIMEOUT),
        };
        if let Err(e) = self.device.poll(poll) {
            log::warn!("relecture GPU impossible : {e}");
            return None;
        }
        receiver.recv_timeout(TIMEOUT).ok()?.ok()?;
        let rgba = {
            let mapped = self.buffer.get_mapped_range(..).ok()?;
            unpack(&mapped, self.size, self.row_bytes, self.bgra)
        };
        self.buffer.unmap();
        Some(rgba)
    }
}

/// Retire l'alignement des lignes et remet les canaux dans l'ordre RGBA.
fn unpack(data: &[u8], [width, height]: [u32; 2], row_bytes: u32, bgra: bool) -> Vec<u8> {
    let width = width as usize * 4;
    let mut rgba = Vec::with_capacity(width * height as usize);
    for row in data.chunks_exact(row_bytes as usize).take(height as usize) {
        let row = &row[..width];
        if bgra {
            rgba.extend(
                row.as_chunks::<4>()
                    .0
                    .iter()
                    .flat_map(|p| [p[2], p[1], p[0], p[3]]),
            );
        } else {
            rgba.extend_from_slice(row);
        }
    }
    rgba
}

impl Renderer {
    /// Prépare `scene` et la dessine hors écran dans une image de `size`
    /// pixels (MSAA du moteur, résolu), puis lance sa relecture. La
    /// commande est soumise immédiatement : la préparation de l'image
    /// suivante ne l'affecte pas. `None` si le format du moteur n'est pas
    /// lisible (voir [`Readback::copy`]).
    pub fn snapshot(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &Scene,
        size: [u32; 2],
    ) -> Option<Readback> {
        let format = self.targets.format();
        channel_order(format)?;
        let size = [size[0].max(1), size[1].max(1)];
        self.prepare(device, queue, scene);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("snapshot"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        // Attachements propres à la miniature : ceux de l'écran gardent
        // leur taille (pas de recréation à chaque miniature).
        let mut targets = RenderTargets::new(format, self.targets.samples());
        targets.ensure(device, size);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("snapshot"),
        });
        self.draw(&mut encoder, targets.views(), &view, size);
        let readback = Readback::copy(device, &mut encoder, &texture);
        queue.submit([encoder.finish()]);
        readback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpack_drops_padding_and_swaps_channels() {
        // Deux pixels par ligne, lignes alignées sur 12 octets.
        let data = [
            1, 2, 3, 4, 5, 6, 7, 8, 0, 0, 0, 0, //
            9, 10, 11, 12, 13, 14, 15, 16, 0, 0, 0, 0,
        ];
        assert_eq!(
            unpack(&data, [2, 2], 12, false),
            [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        );
        assert_eq!(
            unpack(&data, [2, 2], 12, true),
            [3, 2, 1, 4, 7, 6, 5, 8, 11, 10, 9, 12, 15, 14, 13, 16]
        );
    }

    #[test]
    fn only_8_bit_color_formats_are_readable() {
        assert_eq!(channel_order(wgpu::TextureFormat::Bgra8Unorm), Some(true));
        assert_eq!(channel_order(wgpu::TextureFormat::Rgba8UnormSrgb), Some(false));
        assert_eq!(channel_order(wgpu::TextureFormat::Rgba16Float), None);
    }
}
