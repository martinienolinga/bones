//! Transitions entre la galerie et l'éditeur, comme dans Procreate : à
//! l'ouverture, la vignette grandit depuis sa carte jusqu'à sa place exacte
//! dans la vue 3D pendant que la galerie s'efface, puis s'efface à son tour
//! sur l'éditeur ; au retour, la vignette fraîche rétrécit depuis sa place
//! dans la vue jusqu'à sa carte.

use std::cell::Cell;

use egui::{Color32, CornerRadius, Rect, TextureHandle};

/// Durée du déplacement, puis du fondu final de l'ouverture (secondes).
const MOVE: f64 = 0.34;
const FADE: f64 = 0.2;
/// Arrondi des vignettes de la galerie.
const CARD_RADIUS: f32 = 7.0;

pub struct Zoom {
    texture: TextureHandle,
    from: Rect,
    to: Rect,
    /// Début, fixé à la première image dessinée (l'heure d'egui de l'image
    /// en cours n'est connue qu'à ce moment-là).
    start: Cell<Option<f64>>,
    /// Fond de la vue 3D, qui recouvre peu à peu la galerie.
    backdrop: Color32,
    /// Ouverture (vers l'éditeur) ou retour (vers la carte).
    pub opening: bool,
}

/// Place de la vignette dans la vue 3D de l'éditeur. La caméra a un champ
/// vertical fixe et la vignette en montre le centre agrandi `crop` fois :
/// elle est centrée, `crop` fois moins haute que l'écran.
pub fn in_view(screen: Rect, texture: &TextureHandle, crop: f32) -> Rect {
    let [w, h] = texture.size();
    let aspect = if h > 0 { w as f32 / h as f32 } else { 4.0 / 3.0 };
    let height = screen.height() / crop.max(0.1);
    Rect::from_center_size(screen.center(), egui::vec2(height * aspect, height))
}

impl Zoom {
    /// Ouverture depuis la carte `card` ; `view` : place de la vignette dans
    /// la vue 3D (voir [`in_view`]), `backdrop` : fond de la vue 3D.
    pub fn open(texture: TextureHandle, card: Rect, view: Rect, backdrop: Color32) -> Self {
        Self {
            texture,
            from: card,
            to: view,
            start: Cell::new(None),
            backdrop,
            opening: true,
        }
    }

    /// Retour depuis la vue 3D vers la carte `card`.
    pub fn close(texture: TextureHandle, view: Rect, card: Rect, backdrop: Color32) -> Self {
        Self {
            texture,
            from: view,
            to: card,
            start: Cell::new(None),
            backdrop,
            opening: false,
        }
    }

    /// Le déplacement est terminé (l'éditeur peut s'afficher dessous).
    pub fn moved(&self, now: f64) -> bool {
        self.elapsed(now) >= MOVE
    }

    /// Plus rien à dessiner.
    pub fn finished(&self, now: f64) -> bool {
        self.elapsed(now) >= if self.opening { MOVE + FADE } else { MOVE }
    }

    /// Temps écoulé depuis la première image (0 avant).
    fn elapsed(&self, now: f64) -> f64 {
        self.start.get().map_or(0.0, |start| now - start)
    }

    /// Dessine le fond et l'image à leur état animé, au-dessus de tout.
    pub fn paint(&self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        if self.start.get().is_none() {
            self.start.set(Some(now));
        }
        if self.finished(now) {
            return;
        }
        let elapsed = self.elapsed(now);
        let t = (elapsed / MOVE).clamp(0.0, 1.0) as f32;
        // Départ vif, arrivée douce.
        let e = 1.0 - (1.0 - t).powi(3);
        let rect = Rect::from_min_max(
            self.from.min.lerp(self.to.min, e),
            self.from.max.lerp(self.to.max, e),
        );
        let (start_radius, end_radius) = if self.opening {
            (CARD_RADIUS, 0.0)
        } else {
            (0.0, CARD_RADIUS)
        };
        let radius = start_radius + (end_radius - start_radius) * e;
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Debug, egui::Id::unique("zoom")));
        // Fond de la vue 3D : il couvre la galerie à l'ouverture (puis laisse
        // place à l'éditeur), et la découvre au retour.
        let cover = match (self.opening, self.moved(now)) {
            (true, false) => e,
            (true, true) => 0.0,
            (false, _) => 1.0 - e,
        };
        if cover > 0.0 {
            painter.rect_filled(ctx.content_rect(), 0, self.backdrop.gamma_multiply(cover));
        }
        let fade = ((elapsed - MOVE) / FADE).clamp(0.0, 1.0) as f32;
        let alpha = if self.opening { 1.0 - fade } else { 1.0 };
        let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        let tint = Color32::WHITE.gamma_multiply(alpha);
        let image = egui::epaint::RectShape::filled(rect, CornerRadius::from(radius), tint);
        painter.add(image.with_texture(self.texture.id(), uv));
        ctx.request_repaint();
    }
}
