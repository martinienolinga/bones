//! Flou d'arrière-plan pour le « verre dépoli » de l'interface (matériau
//! sombre d'iOS) : seules les zones situées sous les panneaux vitrés sont
//! copiées depuis l'image rendue, puis floutées par l'algorithme « dual
//! Kawase » (quatre réductions, trois agrandissements) jusqu'à la demi
//! résolution. Le coût est donc proportionnel à la taille des panneaux, pas
//! à celle de l'écran, et nul quand aucun panneau n'en demande.
//!
//! Le résultat de chaque zone est une texture au format de la source, que
//! l'application confie à egui comme texture native.

use crate::blur_targets::{Chain, Kernels};

/// Zone à flouter, en pixels de la texture source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Region {
    /// Zone ramenée dans une source de taille `size` ; `None` si elle est
    /// vide ou trop petite pour être floutée.
    fn clamped(self, size: [u32; 2]) -> Option<Self> {
        let x = self.x.min(size[0]);
        let y = self.y.min(size[1]);
        let width = self.width.min(size[0] - x);
        let height = self.height.min(size[1] - y);
        (width >= 4 && height >= 4).then_some(Self { x, y, width, height })
    }
}

/// Flou des zones vitrées, une chaîne de textures par zone (réutilisée tant
/// que la taille de la zone ne change pas).
pub struct Backdrop {
    kernels: Kernels,
    chains: Vec<Option<Chain>>,
}

impl Backdrop {
    /// `format` : format de l'image source (celui de la surface).
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self {
            kernels: Kernels::new(device, format),
            chains: Vec::new(),
        }
    }

    /// Enregistre le flou de chaque zone de `source` (qui doit permettre
    /// `COPY_SRC` et avoir le format donné à [`Self::new`]). La zone `i`
    /// produit la sortie `i` ; une zone invalide laisse sa sortie vide.
    pub fn blur(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::Texture,
        regions: &[Region],
    ) {
        let size = [source.width(), source.height()];
        if self.chains.len() < regions.len() {
            self.chains.resize_with(regions.len(), || None);
        }
        for (slot, region) in regions.iter().enumerate() {
            let Some(region) = region.clamped(size) else {
                self.chains[slot] = None;
                continue;
            };
            let chain = &mut self.chains[slot];
            if chain
                .as_ref()
                .is_none_or(|c| c.size() != [region.width, region.height])
            {
                let generation = chain.as_ref().map_or(0, |c| c.generation() + 1);
                *chain = Some(Chain::new(
                    device,
                    &self.kernels,
                    [region.width, region.height],
                    generation,
                ));
            }
            if let Some(chain) = chain {
                chain.record(encoder, &self.kernels, source, region);
            }
        }
    }

    /// Résultat flouté de la zone `slot` et son numéro de génération, qui
    /// change chaque fois que la texture est recréée (nouvelle taille).
    pub fn output(&self, slot: usize) -> Option<(&wgpu::TextureView, u64)> {
        let chain = self.chains.get(slot)?.as_ref()?;
        Some((chain.output_view(), chain.generation()))
    }

    /// Texture de sortie de la zone `slot` (relecture dans les tests).
    pub fn output_texture(&self, slot: usize) -> Option<&wgpu::Texture> {
        Some(self.chains.get(slot)?.as_ref()?.output_texture())
    }
}

#[cfg(test)]
mod tests {
    use super::Region;

    #[test]
    fn regions_are_clamped_to_the_source() {
        let r = Region {
            x: 90,
            y: 10,
            width: 40,
            height: 20,
        };
        let c = r.clamped([100, 100]).unwrap();
        assert_eq!((c.x, c.width, c.height), (90, 10, 20));
        assert!(Region { x: 98, ..r }.clamped([100, 100]).is_none());
        assert!(Region { x: 200, ..r }.clamped([100, 100]).is_none());
    }
}
