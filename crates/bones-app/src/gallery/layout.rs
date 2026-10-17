//! Mise en page de la galerie, comme celle de Procreate : marges et
//! espacements généreux, colonnes selon la largeur (5 en paysage sur un
//! iPad 12,9 pouces, 4 en portrait, 3 sur les petits écrans), vignettes 4:3
//! au-dessus d'un titre et d'un sous-titre centrés.

use egui::{Rect, Vec2, pos2, vec2};

/// Hauteur de la barre du haut (titre, capsule, « + »).
pub const BAR_H: f32 = 76.0;
/// Espace entre la barre et la première rangée.
pub const GRID_TOP: f32 = 22.0;
/// Largeur visée d'une vignette.
const TARGET: f32 = 188.0;
/// Espace minimal entre deux vignettes.
const MIN_GAP: f32 = 60.0;
/// Espace entre deux vignettes, relatif à leur largeur.
const GAP_RATIO: f32 = 0.45;
/// Hauteur du titre et du sous-titre sous la vignette.
pub const CAPTION_H: f32 = 52.0;
/// Espace entre le sous-titre et la rangée suivante.
const ROW_GAP: f32 = 30.0;

/// Grille calculée pour une largeur d'écran.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub columns: usize,
    /// Marge latérale (aussi celle du titre de la barre).
    pub margin: f32,
    /// Zone de la vignette (4:3).
    pub thumb: Vec2,
    /// Pas entre deux cartes.
    pub pitch: Vec2,
}

/// Marge latérale de l'écran, proportionnelle à sa largeur.
pub fn margin(width: f32) -> f32 {
    (width * 0.03).clamp(20.0, 44.0)
}

impl Grid {
    pub fn new(width: f32) -> Self {
        let margin = margin(width);
        let inner = (width - 2.0 * margin).max(TARGET);
        let columns = ((inner + MIN_GAP) / (TARGET + MIN_GAP)).floor().clamp(2.0, 6.0) as usize;
        let n = columns as f32;
        let w = inner / (n + GAP_RATIO * (n - 1.0));
        let thumb = vec2(w, (w * 0.75).round());
        Self {
            columns,
            margin,
            thumb,
            pitch: vec2(w * (1.0 + GAP_RATIO), thumb.y + CAPTION_H + ROW_GAP),
        }
    }

    pub fn rows(&self, cards: usize) -> usize {
        cards.div_ceil(self.columns)
    }

    /// Hauteur totale de `cards` cartes.
    pub fn height(&self, cards: usize) -> f32 {
        self.rows(cards) as f32 * self.pitch.y
    }

    /// Vignette de la carte `index`, relative à `origin` (coin haut gauche
    /// de la grille).
    pub fn thumb_rect(&self, origin: egui::Pos2, index: usize) -> Rect {
        let (row, col) = (index / self.columns, index % self.columns);
        let min = pos2(
            origin.x + self.margin + col as f32 * self.pitch.x,
            origin.y + row as f32 * self.pitch.y,
        );
        Rect::from_min_size(min, self.thumb)
    }

    /// Carte entière (vignette et légende), avec la moitié de l'espace
    /// entre colonnes de chaque côté : le titre peut y déborder.
    pub fn card_rect(&self, origin: egui::Pos2, index: usize) -> Rect {
        let thumb = self.thumb_rect(origin, index);
        let extra = (self.pitch.x - self.thumb.x) * 0.5;
        Rect::from_min_max(
            pos2(thumb.left() - extra, thumb.top()),
            pos2(thumb.right() + extra, thumb.bottom() + CAPTION_H),
        )
    }

    /// Cartes au moins en partie visibles quand la grille défile de
    /// `offset` dans une fenêtre de hauteur `height`.
    pub fn visible(&self, cards: usize, offset: f32, height: f32) -> std::ops::Range<usize> {
        let first = (offset / self.pitch.y).floor().max(0.0) as usize;
        let last = ((offset + height) / self.pitch.y).ceil().max(0.0) as usize;
        (first * self.columns).min(cards)..(last * self.columns).min(cards)
    }
}

/// Image de taille `image` centrée dans `bounds` sans déformation.
pub fn fit(bounds: Rect, image: Vec2) -> Rect {
    if image.x <= 0.0 || image.y <= 0.0 {
        return bounds;
    }
    let scale = (bounds.width() / image.x).min(bounds.height() / image.y);
    Rect::from_center_size(bounds.center(), image * scale)
}
