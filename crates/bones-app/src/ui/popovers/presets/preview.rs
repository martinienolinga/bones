//! Aperçu animé des cartes : chaque préréglage est cuit une fois
//! ([`bones_core::presets::Preview`]), puis dessiné en squelette de bâtons à
//! l'instant voulu, sans toucher au projet. Les aperçus sont construits
//! paresseusement (quelques-uns par image) pour ne jamais bloquer le fil de
//! l'interface.

use std::collections::HashMap;

use bones_core::presets::{Lr, Preset, Preview};
use bones_ui::theme;
use egui::{Color32, Painter, Pos2, Rect, Stroke};
use glam::Vec3;

/// Aperçu prêt à dessiner (cuisson + côté de chaque os et cadrage).
struct Baked {
    preview: Preview,
    sides: Vec<Option<Lr>>,
    lo: Vec3,
    hi: Vec3,
    /// Tampon des segments, réutilisé d'une image à l'autre.
    segs: Vec<[Vec3; 2]>,
}

#[derive(Default)]
pub struct PreviewCache {
    /// `None` : cuisson échouée (squelette vide…) ; inutile de réessayer.
    baked: HashMap<String, Option<Baked>>,
}

impl PreviewCache {
    /// Oublie tous les aperçus (changement de bibliothèque).
    pub fn clear(&mut self) {
        self.baked.clear();
    }

    /// Cuit au plus `budget` aperçus manquants parmi `wanted`. Renvoie vrai
    /// s'il en reste à cuire : l'appelant redemande alors une image.
    pub fn warm(&mut self, wanted: &[&Preset], budget: usize) -> bool {
        let mut done = 0;
        for preset in wanted {
            if self.baked.contains_key(&preset.info.id) {
                continue;
            }
            if done >= budget {
                return true;
            }
            self.baked.insert(preset.info.id.clone(), Baked::new(preset));
            done += 1;
        }
        false
    }

    /// Dessine l'aperçu de `preset` dans `rect` à l'instant `time`. Renvoie
    /// faux tant que l'aperçu n'est pas cuit (l'appelant dessine un repère).
    pub fn draw(&mut self, painter: &Painter, rect: Rect, preset: &Preset, time: f32) -> bool {
        match self.baked.get_mut(&preset.info.id) {
            None => false,
            Some(None) => true,
            Some(Some(baked)) => {
                baked.paint(painter, rect, time);
                true
            }
        }
    }
}

impl Baked {
    fn new(preset: &Preset) -> Option<Baked> {
        let preview = Preview::new(preset).ok()?;
        let (lo, hi) = preview.bounds();
        if !lo.is_finite() || !hi.is_finite() {
            return None;
        }
        let sides = preview.sides();
        Some(Baked {
            preview,
            sides,
            lo,
            hi,
            segs: Vec::new(),
        })
    }

    fn paint(&mut self, painter: &Painter, rect: Rect, time: f32) {
        self.preview.segments(time, &mut self.segs);
        // Projection orthographique de face (X à droite, Y vers le haut),
        // cadrée sur la boîte de tout le mouvement pour que la silhouette
        // reste dans la carte pendant le cycle.
        let span = (self.hi - self.lo).max(Vec3::splat(1e-3));
        let s = span.x.max(span.y);
        let box_ = rect.shrink(rect.height() * 0.12);
        let scale = box_.width().min(box_.height()) / s;
        let center = (self.lo + self.hi) * 0.5;
        let project = |p: Vec3| {
            Pos2::new(
                box_.center().x + (p.x - center.x) * scale,
                box_.center().y - (p.y - center.y) * scale,
            )
        };
        for (i, [a, b]) in self.segs.iter().enumerate() {
            let color = side_color(self.sides.get(i).copied().flatten());
            painter.line_segment([project(*a), project(*b)], Stroke::new(2.0, color));
        }
        // Têtes d'os : petits points clairs, comme les articulations du rendu.
        for [a, _] in &self.segs {
            painter.circle_filled(project(*a), 1.5, Color32::from_white_alpha(140));
        }
    }
}

/// Couleur d'un os selon son côté (mêmes teintes que la vue 3D).
fn side_color(side: Option<Lr>) -> Color32 {
    match side {
        Some(Lr::Left) => theme::BONE_LEFT,
        Some(Lr::Right) => theme::BONE_RIGHT,
        None => theme::BONE_CENTER,
    }
}
