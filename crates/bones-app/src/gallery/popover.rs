//! Fenêtres de la galerie, comme celles de l'éditeur : panneau #2C2C2E aux
//! coins de 20 pt et encoche pointée vers ce qui l'a ouverte, au-dessous
//! (ou au-dessus s'il manque de place). Un voile transparent capte les
//! touchers à l'extérieur, qui ferment la fenêtre sans rien déclencher
//! d'autre.

use bones_ui::theme;
use egui::{Color32, Id, Order, Pos2, Rect, Sense, pos2};

/// Hauteur de l'encoche.
const NOTCH: f32 = 10.0;
/// Marge minimale entre la fenêtre et les bords de l'écran.
const EDGE: f32 = 12.0;

/// Voile plein écran sous les fenêtres ; vrai s'il a été touché.
/// `dim` : opacité du noir (0 : invisible).
pub fn scrim(ctx: &egui::Context, id: &str, dim: f32) -> bool {
    let screen = ctx.content_rect();
    egui::Area::new(Id::unique(id))
        .fixed_pos(screen.min)
        .order(Order::Foreground)
        .show(ctx, |ui| {
            let response = ui.allocate_rect(screen, Sense::click());
            if dim > 0.0 {
                ui.painter()
                    .rect_filled(screen, 0, Color32::from_black_alpha((255.0 * dim) as u8));
            }
            response.clicked() || response.secondary_clicked()
        })
        .inner
}

/// Fenêtre de largeur `width` ancrée sur `anchor`, marge intérieure
/// `margin` ; retourne sa zone.
pub fn show(
    ctx: &egui::Context,
    id: &str,
    anchor: Rect,
    width: f32,
    margin: i8,
    add: impl FnOnce(&mut egui::Ui),
) -> Rect {
    let id = Id::unique(id);
    let screen = ctx.content_rect();
    let t = ctx.animate_bool_with_time(id.with("anim"), true, 0.16);
    // Hauteur de l'image précédente (placement au-dessus si nécessaire).
    let height = ctx.data(|d| d.get_temp::<f32>(id.with("h"))).unwrap_or(200.0);
    let below = anchor.bottom() + NOTCH + height + EDGE <= screen.bottom()
        || anchor.top() - NOTCH - height < screen.top();
    let left = (anchor.center().x - width * 0.5).clamp(screen.left() + EDGE, screen.right() - width - EDGE);
    let slide = 8.0 * (1.0 - t);
    let top = if below {
        anchor.bottom() + NOTCH + 4.0 - slide
    } else {
        anchor.top() - NOTCH - 4.0 - height + slide
    };
    let max_height = if below {
        screen.bottom() - top - EDGE
    } else {
        anchor.top() - screen.top() - NOTCH - EDGE
    };
    let rect = egui::Area::new(id)
        .fixed_pos(pos2(left, top))
        .order(Order::Tooltip)
        .show(ctx, |ui| {
            ui.set_opacity(t);
            let inner = 2.0 * f32::from(margin);
            let frame = theme::panel_frame().inner_margin(egui::Margin::same(margin));
            let panel = frame.show(ui, |ui| {
                ui.set_width(width - inner);
                ui.set_max_height(max_height - inner);
                add(ui);
            });
            let rect = panel.response.rect;
            let x = anchor.center().x.clamp(rect.left() + 30.0, rect.right() - 30.0);
            ui.painter().add(egui::Shape::convex_polygon(
                notch(rect, x, below),
                theme::PANEL,
                egui::Stroke::NONE,
            ));
            rect
        })
        .inner;
    ctx.data_mut(|d| d.insert_temp(id.with("h"), rect.height()));
    rect
}

/// Triangle de l'encoche, pointe vers l'ancre.
fn notch(rect: Rect, x: f32, below: bool) -> Vec<Pos2> {
    if below {
        vec![
            pos2(x - 12.0, rect.top() + 1.0),
            pos2(x, rect.top() - NOTCH),
            pos2(x + 12.0, rect.top() + 1.0),
        ]
    } else {
        vec![
            pos2(x + 12.0, rect.bottom() - 1.0),
            pos2(x, rect.bottom() + NOTCH),
            pos2(x - 12.0, rect.bottom() - 1.0),
        ]
    }
}

/// Ligne de titre d'une fenêtre (24 pt semi-gras).
pub fn header(ui: &mut egui::Ui, title: &str) {
    ui.label(egui::RichText::new(title).font(theme::semibold(24.0)));
    ui.add_space(4.0);
}

/// Oublie l'animation d'ouverture (la prochaine ouverture la rejoue).
pub fn reset(ctx: &egui::Context, id: &str) {
    ctx.animate_bool_with_time(Id::unique(id).with("anim"), false, 0.0);
}
