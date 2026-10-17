//! Réglages de la peinture de poids, calqués sur le mode « Weight Paint » de
//! Blender : outils, modes de mélange, courbes d'atténuation, pinceau et
//! options du mode.

use serde::{Deserialize, Serialize};

/// Mode de mélange du pinceau (menu « Blend » de Blender).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Blend {
    /// Tend vers le poids du pinceau.
    #[default]
    Mix,
    /// Ajoute le poids du pinceau.
    Add,
    /// Retire le poids du pinceau.
    Subtract,
    /// Ne fait qu'augmenter les poids plus faibles que celui du pinceau.
    Lighten,
    /// Ne fait que diminuer les poids plus forts que celui du pinceau.
    Darken,
    /// Multiplie par le poids du pinceau.
    Multiply,
}

impl Blend {
    pub const ALL: [Self; 6] = [
        Self::Mix,
        Self::Add,
        Self::Subtract,
        Self::Lighten,
        Self::Darken,
        Self::Multiply,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Mix => "Mix",
            Self::Add => "Add",
            Self::Subtract => "Subtract",
            Self::Lighten => "Lighten",
            Self::Darken => "Darken",
            Self::Multiply => "Multiply",
        }
    }
}

/// Outil de peinture (barre d'outils du mode « Weight Paint »).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaintTool {
    /// Peint le poids du pinceau.
    #[default]
    Draw,
    /// Lisse les poids avec ceux des sommets voisins.
    Blur,
    /// Tend vers la moyenne des poids sous le pinceau.
    Average,
    /// Étale les poids dans le sens du trait.
    Smear,
    /// Dégradé du début à la fin du geste.
    Gradient,
    /// Pipette : le poids sous le doigt devient celui du pinceau.
    Sample,
}

impl PaintTool {
    pub const ALL: [Self; 6] = [
        Self::Draw,
        Self::Blur,
        Self::Average,
        Self::Smear,
        Self::Gradient,
        Self::Sample,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Draw => "Draw",
            Self::Blur => "Blur",
            Self::Average => "Average",
            Self::Smear => "Smear",
            Self::Gradient => "Gradient",
            Self::Sample => "Sample",
        }
    }
}

/// Forme du dégradé.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GradientKind {
    #[default]
    Linear,
    Radial,
}

impl GradientKind {
    pub const ALL: [Self; 2] = [Self::Linear, Self::Radial];

    pub fn label(self) -> &'static str {
        match self {
            Self::Linear => "Linear",
            Self::Radial => "Radial",
        }
    }
}

/// Courbe d'atténuation du pinceau, du centre (1) au bord (0) : mêmes
/// préréglages que Blender (`BKE_brush_curve_strength`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Falloff {
    /// `3p² − 2p³` (smoothstep).
    #[default]
    Smooth,
    /// `√(2p − p²)` : quart de cercle.
    Sphere,
    /// `√p`.
    Root,
    /// `p²`.
    Sharp,
    /// `p`.
    Linear,
    /// 1 partout.
    Constant,
}

impl Falloff {
    pub const ALL: [Self; 6] = [
        Self::Smooth,
        Self::Sphere,
        Self::Root,
        Self::Sharp,
        Self::Linear,
        Self::Constant,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Smooth => "Smooth",
            Self::Sphere => "Sphere",
            Self::Root => "Root",
            Self::Sharp => "Sharp",
            Self::Linear => "Linear",
            Self::Constant => "Constant",
        }
    }

    /// Intensité à la distance relative `t` du centre (0 au centre, 1 au
    /// bord), entre 0 et 1 ; nulle au-delà du bord.
    pub fn strength(self, t: f32) -> f32 {
        if !(t < 1.0) {
            return 0.0;
        }
        let p = 1.0 - t.max(0.0);
        let s = match self {
            Self::Smooth => p * p * (3.0 - 2.0 * p),
            Self::Sphere => (2.0 * p - p * p).max(0.0).sqrt(),
            Self::Root => p.sqrt(),
            Self::Sharp => p * p,
            Self::Linear => p,
            Self::Constant => 1.0,
        };
        s.clamp(0.0, 1.0)
    }
}

/// Réglages du pinceau.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BrushSettings {
    pub tool: PaintTool,
    pub blend: Blend,
    pub gradient: GradientKind,
    /// Poids peint (0 à 1).
    pub weight: f32,
    /// Rayon du pinceau, en points d'écran.
    pub radius: f32,
    /// Force (0 à 1).
    pub strength: f32,
    pub falloff: Falloff,
    /// Ne peint que les faces tournées vers la caméra.
    pub front_faces_only: bool,
    /// Accumulation (aérographe) : chaque touche du pinceau s'ajoute à la
    /// précédente. Sinon, un trait ne dépasse jamais sa force maximale en un
    /// sommet, même en repassant dessus.
    pub accumulate: bool,
    /// Espacement des touches le long du trait, en fraction du diamètre.
    pub spacing: f32,
    /// La pression du crayon module la force.
    pub pressure_strength: bool,
    /// La pression du crayon module le rayon.
    pub pressure_radius: bool,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            tool: PaintTool::Draw,
            blend: Blend::Mix,
            gradient: GradientKind::Linear,
            weight: 1.0,
            radius: 50.0,
            strength: 1.0,
            falloff: Falloff::Smooth,
            front_faces_only: false,
            accumulate: false,
            spacing: 0.1,
            pressure_strength: true,
            pressure_radius: false,
        }
    }
}

/// Réglages du mode « Weight Paint ».
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WeightPaintSettings {
    pub brush: BrushSettings,
    /// Peinture symétrique (« X Mirror ») : l'os symétrique (`.l` ↔ `.r`)
    /// est peint aux sommets symétriques.
    pub mirror_x: bool,
    /// Normalisation automatique : les poids des os déformants d'un sommet
    /// gardent une somme de 1, le poids peint restant tel quel.
    pub auto_normalize: bool,
    /// Affiche en noir les sommets sans aucun poids pour l'os peint.
    pub show_zero_weights: bool,
    /// Affiche les arêtes du maillage.
    pub show_wire: bool,
}

impl Default for WeightPaintSettings {
    fn default() -> Self {
        Self {
            brush: BrushSettings::default(),
            mirror_x: false,
            auto_normalize: true,
            show_zero_weights: false,
            show_wire: false,
        }
    }
}

/// Outils du menu « Weights » de Blender, appliqués à tout le maillage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WeightsTool {
    /// Normalise les poids de tous les sommets (somme 1 par sommet).
    NormalizeAll,
    /// Retire les influences de l'os peint plus faibles que `threshold`.
    Clean { threshold: f32 },
    /// Lisse les poids de l'os peint avec ceux des sommets voisins.
    Smooth { iterations: u32, factor: f32 },
    /// Inverse les poids de l'os peint (`1 − w`).
    Invert,
    /// `w × gain + offset` sur les poids de l'os peint.
    Levels { offset: f32, gain: f32 },
    /// Au plus `max` influences par sommet (les plus faibles sont retirées).
    LimitTotal { max: u8 },
    /// Arrondit les poids de l'os peint à `steps` paliers.
    Quantize { steps: u32 },
    /// Recopie les poids de chaque côté sur le côté symétrique.
    Mirror,
    /// Poids automatiques avec la méthode de liaison courante (remplace les
    /// poids peints).
    Automatic,
}

impl WeightsTool {
    pub fn label(&self) -> &'static str {
        match self {
            Self::NormalizeAll => "Normalize All",
            Self::Clean { .. } => "Clean",
            Self::Smooth { .. } => "Smooth",
            Self::Invert => "Invert",
            Self::Levels { .. } => "Levels",
            Self::LimitTotal { .. } => "Limit Total",
            Self::Quantize { .. } => "Quantize",
            Self::Mirror => "Mirror",
            Self::Automatic => "Automatic",
        }
    }
}
