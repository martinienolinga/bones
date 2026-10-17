//! Guide de rotation 3D, comme dans Blender et Nomad Sculpt : glisser dessus
//! fait tourner la vue, toucher un axe (ou une face du cube) aligne la vue
//! avec une transition animée. Quatre boutons l'accompagnent : verrou de
//! rotation, cadrage, perspective / orthographique et vue d'accueil.

mod axes;
mod cube;
mod view;

use bones_core::Editor;
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Apparence du guide.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GizmoStyle {
    /// Axes colorés à la Blender.
    #[default]
    Axes,
    /// Cube de vue à la Nomad Sculpt.
    Cube,
}

/// Taille de la zone du guide, en points.
const SIZE: f32 = 116.0;
/// Sensibilité de la rotation par glisser sur le guide (rad/point).
const DRAG_SPEED: f32 = 0.012;

/// Couleurs des axes (X rouge, Y vert, Z bleu), communes aux deux styles.
pub(crate) const AXIS_COLORS: [egui::Color32; 3] = [
    egui::Color32::from_rgb(0xF2, 0x4E, 0x5B),
    egui::Color32::from_rgb(0x8C, 0xD9, 0x4A),
    egui::Color32::from_rgb(0x3D, 0x8B, 0xFF),
];

/// Repère écran de la caméra : où apparaît une direction monde dans le guide.
pub(crate) struct Basis {
    right: Vec3,
    up: Vec3,
    toward_viewer: Vec3,
}

impl Basis {
    fn of(editor: &Editor) -> Self {
        let camera = &editor.camera;
        Self {
            right: camera.right(),
            up: camera.up(),
            toward_viewer: -camera.forward(),
        }
    }

    /// Décalage écran (y vers le bas) d'une direction, et sa profondeur
    /// (positive vers l'observateur).
    pub(crate) fn project(&self, v: Vec3) -> (egui::Vec2, f32) {
        (
            egui::vec2(v.dot(self.right), -v.dot(self.up)),
            v.dot(self.toward_viewer),
        )
    }
}

/// Affiche le guide en haut à droite (à gauche de la boîte à outils) et
/// retourne les zones occupées, à exclure des gestes 3D.
pub fn show(ctx: &egui::Context, editor: &mut Editor, style: GizmoStyle) -> Vec<egui::Rect> {
    view::step(ctx, editor);
    let area = egui::Area::new(egui::Id::unique("view-gizmo"))
        .anchor(egui::Align2::RIGHT_TOP, [-90.0, 76.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                gizmo(ui, editor, style);
                buttons(ui, editor);
            });
        });
    vec![area.response.rect]
}

fn gizmo(ui: &mut egui::Ui, editor: &mut Editor, style: GizmoStyle) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(SIZE, SIZE), egui::Sense::click_and_drag());
    let hovered = response.hovered() || response.dragged();
    let glow = ui.ctx().animate_bool_with_time(response.id, hovered, 0.15);
    let painter = ui.painter_at(rect.expand(4.0));
    let center = rect.center();
    // Disque d'arrière-plan pour les axes ; le cube, lui, se suffit.
    let disc = match style {
        GizmoStyle::Axes => 10.0 + 18.0 * glow,
        GizmoStyle::Cube => 14.0 * glow,
    };
    painter.circle_filled(center, SIZE * 0.5, egui::Color32::from_white_alpha(disc as u8));

    let basis = Basis::of(editor);
    let hit = response.interact_pointer_pos();
    let target = match style {
        GizmoStyle::Axes => axes::paint(&painter, center, SIZE * 0.36, &basis, hit),
        GizmoStyle::Cube => cube::paint(&painter, center, SIZE * 0.31, &basis, hit),
    };

    if response.dragged() {
        let d = response.drag_delta();
        view::cancel(ui.ctx());
        editor.camera.yaw -= d.x * DRAG_SPEED;
        editor.camera.pitch = (editor.camera.pitch + d.y * DRAG_SPEED).clamp(-view::PITCH, view::PITCH);
    } else if response.clicked()
        && let Some(direction) = target
    {
        view::animate_to(ui.ctx(), editor, view::angles_for(direction, editor.camera.yaw));
    }
}

fn buttons(ui: &mut egui::Ui, editor: &mut Editor) {
    theme::card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let camera = &mut editor.camera;
            let lock_hint = if camera.locked {
                "Unlock rotation"
            } else {
                "Lock rotation"
            };
            if widgets::tool_button(ui, Icon::Lock, camera.locked, lock_hint).clicked() {
                camera.locked = !camera.locked;
            }
            if widgets::tool_button(ui, Icon::Ortho, camera.orthographic, "Orthographic view").clicked() {
                camera.orthographic = !camera.orthographic;
            }
            if widgets::tool_button(ui, Icon::Frame, false, "Frame the model").clicked() {
                editor.frame_all();
            }
            if widgets::tool_button(ui, Icon::Home, false, "Home view").clicked() {
                editor.frame_all();
                view::animate_to(ui.ctx(), editor, view::HOME);
            }
        });
    });
}
