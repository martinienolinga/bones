//! Crayon vectoriel : dessine sur une grille de conception 24 × 24 mise à
//! l'échelle dans le rectangle de l'icône.

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke};

/// Point de la grille de conception (x, y), y vers le bas.
pub(crate) type P = (f32, f32);

pub(crate) struct Pen<'a> {
    painter: &'a Painter,
    origin: Pos2,
    scale: f32,
    pub(crate) color: Color32,
    stroke: Stroke,
}

impl<'a> Pen<'a> {
    /// Crayon centré dans `rect` (carré inscrit), trait de 1,7 unité.
    pub(crate) fn new(painter: &'a Painter, rect: Rect, color: Color32) -> Self {
        let side = rect.width().min(rect.height());
        let scale = side / 24.0;
        let origin = rect.center() - egui::vec2(side, side) * 0.5;
        Self {
            painter,
            origin,
            scale,
            color,
            stroke: Stroke::new((1.7 * scale).max(1.0), color),
        }
    }

    /// Même crayon, couleur atténuée (éléments secondaires).
    pub(crate) fn dimmed(&self) -> Pen<'a> {
        let color = self.color.gamma_multiply(0.45);
        Pen {
            color,
            stroke: Stroke::new(self.stroke.width, color),
            ..*self
        }
    }

    fn p(&self, (x, y): P) -> Pos2 {
        self.origin + egui::vec2(x, y) * self.scale
    }

    fn pts(&self, pts: &[P]) -> Vec<Pos2> {
        pts.iter().map(|&p| self.p(p)).collect()
    }

    /// Ligne brisée ouverte.
    pub(crate) fn line(&self, pts: &[P]) {
        self.painter.add(Shape::line(self.pts(pts), self.stroke));
    }

    /// Ligne brisée fermée.
    pub(crate) fn closed(&self, pts: &[P]) {
        self.painter.add(Shape::closed_line(self.pts(pts), self.stroke));
    }

    /// Polygone convexe plein.
    pub(crate) fn fill(&self, pts: &[P]) {
        self.painter
            .add(Shape::convex_polygon(self.pts(pts), self.color, Stroke::NONE));
    }

    pub(crate) fn circle(&self, c: P, r: f32) {
        self.painter.circle_stroke(self.p(c), r * self.scale, self.stroke);
    }

    pub(crate) fn dot(&self, c: P, r: f32) {
        self.painter.circle_filled(self.p(c), r * self.scale, self.color);
    }

    /// Rectangle (contour) aux coins arrondis.
    pub(crate) fn rect(&self, min: P, max: P, r: f32) {
        let rect = Rect::from_min_max(self.p(min), self.p(max));
        self.painter.rect_stroke(
            rect,
            (r * self.scale) as u8,
            self.stroke,
            egui::StrokeKind::Middle,
        );
    }

    /// Rectangle plein aux coins arrondis.
    pub(crate) fn rect_filled(&self, min: P, max: P, r: f32) {
        let rect = Rect::from_min_max(self.p(min), self.p(max));
        self.painter.rect_filled(rect, (r * self.scale) as u8, self.color);
    }

    /// Points d'un arc ; angles en degrés, 0° vers +x, sens horaire à l'écran.
    pub(crate) fn arc_points(c: P, r: f32, from: f32, to: f32) -> Vec<P> {
        let n = (((to - from).abs() / 8.0).ceil() as usize).max(4);
        (0..=n)
            .map(|i| {
                let a = (from + (to - from) * i as f32 / n as f32).to_radians();
                (c.0 + r * a.cos(), c.1 + r * a.sin())
            })
            .collect()
    }

    pub(crate) fn arc(&self, c: P, r: f32, from: f32, to: f32) {
        self.line(&Self::arc_points(c, r, from, to));
    }

    /// Pointe de flèche pleine en `tip`, orientée selon `dir_deg`.
    pub(crate) fn arrow_head(&self, tip: P, dir_deg: f32, size: f32) {
        let a = dir_deg.to_radians();
        let back = |da: f32| {
            let b = a + std::f32::consts::PI + da;
            (tip.0 + size * b.cos(), tip.1 + size * b.sin())
        };
        self.fill(&[tip, back(0.5), back(-0.5)]);
    }

    /// Ligne pointillée (petits points pleins).
    pub(crate) fn dotted(&self, pts: &[P], spacing: f32) {
        let shapes = Shape::dotted_line(
            &self.pts(pts),
            self.color,
            spacing * self.scale,
            self.stroke.width * 0.6,
        );
        self.painter.extend(shapes);
    }

    /// Courbe y = f(x) échantillonnée entre `x0` et `x1`.
    pub(crate) fn curve_points(x0: f32, x1: f32, f: impl Fn(f32) -> f32) -> Vec<P> {
        (0..=16)
            .map(|i| {
                let x = x0 + (x1 - x0) * i as f32 / 16.0;
                (x, f(x))
            })
            .collect()
    }

    /// Contour d'un « stade » (capsule 2D) entre `a` et `b`, de rayon `r`.
    pub(crate) fn stadium(&self, a: P, b: P, r: f32) {
        let ang = (b.1 - a.1).atan2(b.0 - a.0).to_degrees();
        let mut pts = Self::arc_points(b, r, ang - 90.0, ang + 90.0);
        pts.extend(Self::arc_points(a, r, ang + 90.0, ang + 270.0));
        self.closed(&pts);
    }
}
