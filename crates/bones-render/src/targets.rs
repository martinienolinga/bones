//! Attachements de rendu intermédiaires (profondeur, couleur multi-échantillonnée).
//!
//! Ils sont déclarés `TRANSIENT_ATTACHMENT` : sur les GPU Apple (iPad), ils
//! vivent uniquement dans la mémoire de tuile (« memoryless ») et ne
//! consomment aucune mémoire vidéo, même avec le MSAA 4×.

use crate::pipelines::DEPTH_FORMAT;

pub(crate) struct RenderTargets {
    format: wgpu::TextureFormat,
    samples: u32,
    size: [u32; 2],
    views: Option<Views>,
}

pub(crate) struct Views {
    pub depth: wgpu::TextureView,
    /// Couleur multi-échantillonnée, résolue dans la cible finale.
    pub msaa: Option<wgpu::TextureView>,
}

impl RenderTargets {
    pub fn new(format: wgpu::TextureFormat, samples: u32) -> Self {
        Self {
            format,
            samples,
            size: [0, 0],
            views: None,
        }
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub fn samples(&self) -> u32 {
        self.samples
    }

    /// Vues créées par le dernier [`Self::ensure`].
    pub fn views(&self) -> &Views {
        self.views.as_ref().expect("RenderTargets::ensure appelé avant")
    }

    /// Crée (ou recrée si la taille a changé) les vues pour une cible de
    /// taille `size`.
    pub fn ensure(&mut self, device: &wgpu::Device, size: [u32; 2]) {
        let size = [size[0].max(1), size[1].max(1)];
        if self.size != size || self.views.is_none() {
            self.size = size;
            let create = |label, format| {
                device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some(label),
                        size: wgpu::Extent3d {
                            width: size[0],
                            height: size[1],
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: self.samples,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TRANSIENT_ATTACHMENT,
                        view_formats: &[],
                    })
                    .create_view(&wgpu::TextureViewDescriptor::default())
            };
            self.views = Some(Views {
                depth: create("depth", DEPTH_FORMAT),
                msaa: (self.samples > 1).then(|| create("msaa-color", self.format)),
            });
        }
    }
}

/// Nombre d'échantillons MSAA : 4 si le format le garantit, sinon 1.
pub(crate) fn best_sample_count(device: &wgpu::Device, format: wgpu::TextureFormat) -> u32 {
    let color = format.guaranteed_format_features(device.features()).flags;
    let depth = DEPTH_FORMAT.guaranteed_format_features(device.features()).flags;
    let resolvable = color.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_RESOLVE);
    if resolvable && color.sample_count_supported(4) && depth.sample_count_supported(4) {
        4
    } else {
        1
    }
}
