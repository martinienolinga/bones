//! Manipulateurs 3D (« gizmos ») de translation, rotation et échelle, pour
//! les os et le modèle entier.
//!
//! Tout est indépendant de la plateforme : [`geometry`] construit les formes
//! à l'écran (polylignes et polygones, en pixels physiques, de taille
//! constante quelle que soit la distance), [`pick`] trouve la partie sous le
//! doigt et [`drag`] convertit un glissement en transformation (relative au
//! début du geste, donc sans dérive).
//!
//! Couleurs : X rouge, Y vert, Z bleu ; anneau de vue gris.

pub mod drag;
pub mod geometry;
pub mod pick;
mod projector;
mod shapes;
#[cfg(test)]
mod tests;

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

pub use drag::{ManipDrag, Snap};
pub use geometry::{Element, Geometry, Kind, Options, Tint};
pub use projector::Projector;

/// Axe du repère du manipulateur.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub const ALL: [Axis; 3] = [Axis::X, Axis::Y, Axis::Z];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn unit(self) -> Vec3 {
        match self {
            Axis::X => Vec3::X,
            Axis::Y => Vec3::Y,
            Axis::Z => Vec3::Z,
        }
    }

    /// Les deux autres axes, dans l'ordre direct (X → (Y, Z), Y → (Z, X)…).
    pub fn others(self) -> (Axis, Axis) {
        match self {
            Axis::X => (Axis::Y, Axis::Z),
            Axis::Y => (Axis::Z, Axis::X),
            Axis::Z => (Axis::X, Axis::Y),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Axis::X => "X",
            Axis::Y => "Y",
            Axis::Z => "Z",
        }
    }
}

/// Partie saisissable d'un manipulateur.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    /// Flèche : translation le long d'un axe.
    Axis(Axis),
    /// Carré : translation dans le plan normal à l'axe.
    Plane(Axis),
    /// Anneau : rotation autour d'un axe.
    Ring(Axis),
    /// Grand anneau gris : rotation autour de l'axe de vue.
    ViewRing,
    /// Intérieur de la sphère : rotation libre (trackball).
    Trackball,
    /// Centre : translation dans le plan de l'écran.
    Center,
    /// Cube au bout d'un axe : échelle le long de cet axe.
    ScaleAxis(Axis),
    /// Cercle d'échelle : échelle uniforme.
    ScaleUniform,
}

impl Part {
    pub fn is_rotation(self) -> bool {
        matches!(self, Part::Ring(_) | Part::ViewRing | Part::Trackball)
    }

    pub fn is_scale(self) -> bool {
        matches!(self, Part::ScaleAxis(_) | Part::ScaleUniform)
    }

    pub fn is_translation(self) -> bool {
        matches!(self, Part::Axis(_) | Part::Plane(_) | Part::Center)
    }
}

/// Jeu de poignées affiché.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManipulatorMode {
    /// Flèches, carrés de plan et centre.
    Translate,
    /// Anneaux d'axes, anneau de vue et trackball.
    Rotate,
    /// Cubes d'axes et cercle d'échelle uniforme.
    Scale,
    /// Flèches, arcs de rotation et carrés de plan réunis.
    #[default]
    Combined,
}

impl ManipulatorMode {
    pub const ALL: [ManipulatorMode; 4] = [
        ManipulatorMode::Translate,
        ManipulatorMode::Rotate,
        ManipulatorMode::Scale,
        ManipulatorMode::Combined,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ManipulatorMode::Translate => "Move",
            ManipulatorMode::Rotate => "Rotate",
            ManipulatorMode::Scale => "Scale",
            ManipulatorMode::Combined => "Transform",
        }
    }
}

/// Repère des poignées : axes du monde ou de l'os.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransformSpace {
    #[default]
    World,
    Local,
}

impl TransformSpace {
    pub fn label(self) -> &'static str {
        match self {
            TransformSpace::World => "Global",
            TransformSpace::Local => "Local",
        }
    }
}

/// Réglages du manipulateur (persistables).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ManipulatorSettings {
    pub mode: ManipulatorMode,
    pub space: TransformSpace,
    /// Magnétisme : angles, distances et échelles par pas.
    pub snap: bool,
    /// Pas angulaire, en degrés.
    pub angle_step_deg: f32,
    /// Pas de déplacement, en unités monde (mètres).
    pub distance_step: f32,
    /// Pas d'échelle.
    pub scale_step: f32,
    /// Rayon des flèches à l'écran, en points.
    pub size: f32,
    pub visible: bool,
    /// Inversion temporaire du magnétisme (touche Maj maintenue).
    #[serde(skip)]
    pub invert_snap: bool,
}

impl Default for ManipulatorSettings {
    fn default() -> Self {
        Self {
            mode: ManipulatorMode::default(),
            space: TransformSpace::default(),
            snap: false,
            angle_step_deg: 15.0,
            distance_step: 0.05,
            scale_step: 0.1,
            size: 96.0,
            visible: true,
            invert_snap: false,
        }
    }
}

impl ManipulatorSettings {
    /// Pas de magnétisme actifs, compte tenu de l'inversion temporaire.
    pub fn active_snap(&self) -> Option<Snap> {
        (self.snap != self.invert_snap).then(|| Snap {
            angle: self.angle_step_deg.max(0.1).to_radians(),
            distance: self.distance_step.max(1e-4),
            scale: self.scale_step.max(1e-3),
        })
    }
}

/// Résultat d'un glissement, relatif au début du geste : translation monde,
/// rotation monde (autour de l'origine du manipulateur) et échelle selon les
/// axes du manipulateur.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ManipDelta {
    pub translate: Vec3,
    pub rotate: Quat,
    pub scale: Vec3,
}

impl ManipDelta {
    pub const IDENTITY: Self = Self {
        translate: Vec3::ZERO,
        rotate: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    pub fn is_identity(&self) -> bool {
        self.translate == Vec3::ZERO && self.rotate == Quat::IDENTITY && self.scale == Vec3::ONE
    }
}

impl Default for ManipDelta {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Repère du manipulateur : origine (pivot) et orientation de ses axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub origin: Vec3,
    pub orientation: Quat,
}

impl Frame {
    pub fn axis(&self, axis: Axis) -> Vec3 {
        (self.orientation * axis.unit()).normalize()
    }
}
