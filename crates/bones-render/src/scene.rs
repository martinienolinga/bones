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

/// Affichage des poids d'un os en carte de chaleur, comme le mode Weight
/// Paint de Blender : bleu (0) → cyan → vert → jaune → rouge (1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WeightView {
    /// Os dont on affiche les poids ; `None` : carte entièrement à zéro.
    pub bone: Option<u32>,
    /// Sommets sans poids pour cet os en noir (« Zero Weights »), sinon bleus.
    pub zero_black: bool,
    /// Arêtes du maillage par-dessus (si le GPU sait tracer en fil de fer).
    pub wire: bool,
}

/// Indice d'os qui ne correspond à aucun joint : carte à zéro partout.
const NO_BONE: f32 = 1.0e9;

/// Uniformes partagés par tous les shaders (voir `Globals` en WGSL).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Globals {
    pub view_proj: [[f32; 4]; 4],
    pub eye: [f32; 4],
    /// x : os en carte de poids (-1 : pas de carte), y : nombre de joints,
    /// z : opacité du maillage, w : matériaux et textures affichés.
    pub params: [f32; 4],
    /// x : poids nuls en noir ; y, z, w : réservés.
    pub paint: [f32; 4],
}

impl Globals {
    pub fn from_scene(scene: &Scene) -> Self {
        let heat = scene.weights.map_or(-1.0, |w| w.bone.map_or(NO_BONE, |b| b as f32));
        let zero_black = scene.weights.is_some_and(|w| w.zero_black);
        Self {
            view_proj: scene.view_proj.to_cols_array_2d(),
            eye: scene.eye.extend(1.0).to_array(),
            params: [
                heat,
                scene.joints.len() as f32,
                1.0,
                f32::from(u8::from(scene.textured)),
            ],
            paint: [f32::from(u8::from(zero_black)), 0.0, 0.0, 0.0],
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
    /// Matériaux et textures du modèle ; sinon rendu « argile » uniforme.
    pub textured: bool,
    /// Carte de poids (prioritaire sur les textures).
    pub weights: Option<WeightView>,
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
        assert_eq!(std::mem::size_of::<Globals>(), 112);
    }
}
