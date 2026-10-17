//! Textures des matériaux : réduction aux limites du GPU, mipmaps calculés
//! sur le CPU (filtre boîte 2×2 en espace linéaire) et envoi au GPU.
//!
//! Les pixels décodés ne vivent que le temps de l'envoi : une fois les
//! niveaux copiés, plus aucune image n'est gardée en mémoire centrale.

/// Image RGBA 8 bits (sRGB).
pub(crate) struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Table sRGB → linéaire (gamma 2,2, cohérent avec `to_srgb` des shaders).
fn to_linear_table() -> [f32; 256] {
    std::array::from_fn(|i| (i as f32 / 255.0).powf(2.2))
}

impl Rgba {
    /// Demi-résolution par moyenne de blocs 2×2, en linéaire (l'alpha est
    /// moyenné tel quel). Les dimensions impaires sont gérées en bordure.
    pub fn half(&self, table: &[f32; 256]) -> Rgba {
        let (w, h) = ((self.width / 2).max(1), (self.height / 2).max(1));
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        for (y, row) in pixels.chunks_exact_mut((w * 4) as usize).enumerate() {
            let y0 = (y as u32 * 2).min(self.height - 1);
            let y1 = (y0 + 1).min(self.height - 1);
            for (x, out) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                let x0 = (x as u32 * 2).min(self.width - 1);
                let x1 = (x0 + 1).min(self.width - 1);
                let texels = [(x0, y0), (x1, y0), (x0, y1), (x1, y1)]
                    .map(|(sx, sy)| ((sy * self.width + sx) * 4) as usize);
                for c in 0..3 {
                    let sum: f32 = texels.iter().map(|i| table[self.pixels[i + c] as usize]).sum();
                    out[c] = ((sum * 0.25).powf(1.0 / 2.2) * 255.0 + 0.5) as u8;
                }
                let alpha: u32 = texels.iter().map(|i| self.pixels[i + 3] as u32).sum();
                out[3] = ((alpha + 2) / 4) as u8;
            }
        }
        Rgba {
            width: w,
            height: h,
            pixels,
        }
    }
}

/// Nombre de niveaux de mipmap d'une image.
pub(crate) fn mip_count(width: u32, height: u32) -> u32 {
    32 - width.max(height).max(1).leading_zeros()
}

/// Envoie l'image et toute sa chaîne de mipmaps ; l'image est consommée.
/// Retourne une vue sRGB (l'échantillonnage renvoie des couleurs linéaires).
pub(crate) fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    mut image: Rgba,
) -> wgpu::TextureView {
    let table = to_linear_table();
    let max = device.limits().max_texture_dimension_2d.max(1);
    while image.width > max || image.height > max {
        image = image.half(&table);
    }
    let levels = mip_count(image.width, image.height);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: image.width,
            height: image.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: levels,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for level in 0..levels {
        if level > 0 {
            image = image.half(&table);
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &image.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(image.width * 4),
                rows_per_image: Some(image.height),
            },
            wgpu::Extent3d {
                width: image.width,
                height: image.height,
                depth_or_array_layers: 1,
            },
        );
    }
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// Texture blanche 1×1 pour les matériaux sans image.
pub(crate) fn white(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::TextureView {
    let image = Rgba {
        width: 1,
        height: 1,
        pixels: vec![255; 4],
    };
    upload(device, queue, "blanc", image)
}

/// Échantillonneur commun : filtrage linéaire, mipmaps, répétition.
pub(crate) fn sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("matériaux"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        anisotropy_clamp: 4,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mip_chain_length() {
        assert_eq!(mip_count(1, 1), 1);
        assert_eq!(mip_count(256, 256), 9);
        assert_eq!(mip_count(300, 7), 9);
    }

    #[test]
    fn half_averages_in_linear_space_and_handles_odd_sizes() {
        let table = to_linear_table();
        // 3×1 : noir, blanc, blanc.
        let image = Rgba {
            width: 3,
            height: 1,
            pixels: vec![0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 0],
        };
        let half = image.half(&table);
        assert_eq!((half.width, half.height), (1, 1));
        // Moyenne linéaire de 0 et 1 (= 0,5) → ≈ 186 en sRGB, pas 128.
        assert!((180..=192).contains(&half.pixels[0]), "{:?}", half.pixels);
        assert_eq!(half.pixels[3], 255);

        let uniform = Rgba {
            width: 4,
            height: 4,
            pixels: [200u8, 100, 50, 255].repeat(16),
        };
        let half = uniform.half(&table);
        assert_eq!((half.width, half.height), (2, 2));
        for p in half.pixels.as_chunks::<4>().0 {
            assert!(p[0].abs_diff(200) <= 1 && p[1].abs_diff(100) <= 1 && p[2].abs_diff(50) <= 1);
        }
    }
}
