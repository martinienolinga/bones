//! Formes du manipulateur à l'écran, en pixels physiques : polylignes et
//! polygones convexes, chacun relié à la partie qu'il représente et à son
//! rôle de couleur. Les éléments sont rangés dans l'ordre de dessin (moitiés
//! arrière des anneaux d'abord).
//!
//! Les tampons sont réutilisés d'une image à l'autre (`clear` puis `build`).

use glam::Vec2;

use super::shapes::Builder;
use super::{Frame, ManipulatorMode, Part};
use crate::manipulator::Axis;
use crate::manipulator::projector::Projector;

/// Rôle de couleur d'un élément (la palette est choisie par l'application).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tint {
    Axis(Axis),
    /// Anneau de vue (gris).
    View,
    /// Poignée de l'anneau de vue (bleu clair).
    Handle,
    /// Boule centrale (gris clair).
    Ball,
    /// Fond du trackball (voile très léger).
    Trackball,
    /// Traits neutres (blanc).
    Neutral,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Polyligne ouverte.
    Line,
    /// Polyligne fermée.
    Loop,
    /// Polygone convexe plein.
    Fill,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Element {
    /// Partie saisie en touchant cet élément (`None` : décoratif).
    pub part: Option<Part>,
    pub tint: Tint,
    pub kind: Kind,
    /// Derrière le centre (moitié arrière d'un anneau) : estompé.
    pub back: bool,
    /// Épaisseur relative du trait (1 : normale).
    pub width: f32,
    /// Toucher l'intérieur d'une boucle saisit aussi la partie.
    pub solid: bool,
    start: u32,
    end: u32,
}

/// Options de construction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Options {
    pub mode: ManipulatorMode,
    /// Longueur des flèches à l'écran, en pixels.
    pub radius_px: f32,
    /// Poignées d'échelle par axe (sinon échelle uniforme seulement).
    pub axis_scale: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Geometry {
    /// Origine projetée.
    pub center: Vec2,
    /// Longueur des flèches, en pixels.
    pub radius: f32,
    points: Vec<Vec2>,
    elements: Vec<Element>,
}

impl Geometry {
    pub fn clear(&mut self) {
        self.points.clear();
        self.elements.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn elements(&self) -> &[Element] {
        &self.elements
    }

    pub fn points(&self, element: &Element) -> &[Vec2] {
        &self.points[element.start as usize..element.end as usize]
    }

    /// Ajoute un élément ; retourne une référence pour régler ses attributs.
    pub(super) fn push(
        &mut self,
        part: Option<Part>,
        tint: Tint,
        kind: Kind,
        points: impl IntoIterator<Item = Vec2>,
    ) -> &mut Element {
        let start = self.points.len() as u32;
        self.points.extend(points);
        self.elements.push(Element {
            part,
            tint,
            kind,
            back: false,
            width: 1.0,
            solid: false,
            start,
            end: self.points.len() as u32,
        });
        self.elements.last_mut().expect("élément ajouté")
    }

    /// Construit le manipulateur ; retourne `false` (géométrie vide) si son
    /// origine est derrière la caméra ou trop proche.
    pub fn build(&mut self, projector: &Projector, frame: &Frame, options: &Options) -> bool {
        self.clear();
        let wpp = projector.world_per_px(frame.origin);
        let world_radius = options.radius_px * wpp;
        let near = (projector.camera().distance * 0.01).max(1e-3);
        if !projector.camera().orthographic && projector.depth(frame.origin) < near + 1.5 * world_radius {
            return false;
        }
        let Some(center) = projector.try_project(frame.origin) else {
            return false;
        };
        self.center = center;
        self.radius = options.radius_px;
        let mut b = Builder::new(self, projector, frame, world_radius, options.radius_px);
        match options.mode {
            ManipulatorMode::Translate => b.translate(),
            ManipulatorMode::Rotate => b.rotate(),
            ManipulatorMode::Scale => b.scale(options.axis_scale),
            ManipulatorMode::Combined => b.combined(),
        }
        true
    }
}
