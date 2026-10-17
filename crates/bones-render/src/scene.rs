//! Données d'une image, telles que l'application les fournit au moteur de rendu.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};

/// Instance d'octaèdre : un os, un fantôme ou une poignée.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct BoneInstance {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
}

impl BoneInstance {
    pub fn new(model: Mat4, color: Vec4) -> Self {
        Self {
            model: model.to_cols_array_2d(),
            color: color.to_array(),
        }
    }
}

/// Sommet de ligne (grille, trajectoires, repères).
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct LineVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

impl LineVertex {
    pub fn new(position: Vec3, color: Vec4) -> Self {
        Self {
            position: position.to_array(),
            color: color.to_array(),
        }
    }
}

/// Uniformes partagés par tous les shaders (voir `Globals` en WGSL).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Globals {
    pub view_proj: [[f32; 4]; 4],
    pub eye: [f32; 4],
    /// x : os en carte de poids (-1 : aucun), y : nombre de joints,
    /// z : opacité du maillage.
    pub params: [f32; 4],
}

impl Globals {
    pub fn from_scene(scene: &Scene) -> Self {
        Self {
            view_proj: scene.view_proj.to_cols_array_2d(),
            eye: scene.eye.extend(1.0).to_array(),
            params: [
                scene.weight_bone.map(|b| b as f32).unwrap_or(-1.0),
                scene.joints.len() as f32,
                1.0,
                0.0,
            ],
        }
    }
}

/// Tout ce qu'il faut pour dessiner une image.
pub struct Scene<'a> {
    pub view_proj: Mat4,
    pub eye: Vec3,
    /// Zone de rendu en pixels physiques : x, y, largeur, hauteur.
    pub viewport: [f32; 4],
    pub background: [f64; 3],
    pub joints: &'a [Mat4],
    pub show_mesh: bool,
    /// Os dont on affiche la carte de poids.
    pub weight_bone: Option<u32>,
    /// Lignes testées en profondeur (grille).
    pub lines: &'a [LineVertex],
    /// Os, fantômes et poignées, dessinés par-dessus le maillage.
    pub bones: &'a [BoneInstance],
    /// Lignes par-dessus tout (trajectoires, pôles IK, centre de masse).
    pub overlay_lines: &'a [LineVertex],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_match_shaders() {
        assert_eq!(std::mem::size_of::<BoneInstance>(), 80);
        assert_eq!(std::mem::size_of::<LineVertex>(), 28);
        assert_eq!(std::mem::size_of::<Globals>(), 96);
    }
}
