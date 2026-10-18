//! Bandeau droit, pendant du bandeau gauche : collé au bord et centré
//! verticalement, il réunit les actions et bascules du mode courant
//! (glyphe + libellé gris, bleus quand la bascule est active).

use bones_core::{Editor, Mode, Side};
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::{BAR_H, UiState, block};

pub fn show(ctx: &egui::Context, editor: &mut Editor, _state: &mut UiState) -> egui::Rect {
    let frame = egui::Frame::new()
        .fill(theme::BAR)
        .corner_radius(egui::CornerRadius {
            nw: 10,
            sw: 10,
            ne: 0,
            se: 0,
        })
        .inner_margin(egui::Margin::symmetric(4, 8));
    block(
        ctx,
        "toolbox",
        egui::Align2::RIGHT_CENTER,
        [0.0, BAR_H * 0.5],
        frame,
        |ui| {
            ui.vertical_centered(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                match editor.mode {
                    Mode::Pose => pose(ui, editor),
                    Mode::Edit => edit(ui, editor),
                    Mode::AutoRig => autorig(ui, editor),
                    Mode::WeightPaint => weights(ui, editor),
                    // Outils du mode Objet : étape 3 (interface).
                    Mode::Object => {}
                }
            });
        },
    )
}

/// Filet de séparation entre deux groupes.
fn divider(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(36.0, 9.0), egui::Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        egui::Stroke::new(0.5, theme::HAIRLINE),
    );
}

fn pose(ui: &mut egui::Ui, editor: &mut Editor) {
    if widgets::tool_cell(ui, Icon::Mirror, "Mirror", false).clicked() {
        editor.mirror_pose();
    }
    if widgets::tool_cell(ui, Icon::Reset, "Reset", false).clicked() {
        let selected_only = editor.selected.is_some();
        editor.reset_pose(selected_only);
    }
    if !editor.rig.pins.is_empty() && widgets::tool_cell(ui, Icon::Close, "Unpin", false).clicked() {
        editor.clear_pins();
    }
    divider(ui);
    let view = &mut editor.view;
    toggle(ui, Icon::Trajectory, "Trail", &mut view.trajectory);
    toggle(ui, Icon::Ghost, "Ghosts", &mut view.ghosts);
    toggle(ui, Icon::CenterOfMass, "Mass", &mut view.center_of_mass);
}

fn edit(ui: &mut egui::Ui, editor: &mut Editor) {
    let selected = editor.selected.is_some();
    let delete = ui
        .add_enabled_ui(selected, |ui| {
            widgets::tool_cell(ui, Icon::Delete, "Delete", false)
        })
        .inner;
    if delete.clicked() {
        editor.delete_selected();
    }
    let duplicate = ui
        .add_enabled_ui(selected, |ui| {
            widgets::tool_cell(ui, Icon::Duplicate, "Copy", false)
        })
        .inner;
    if duplicate.on_hover_text("Duplicate (D)").clicked() {
        editor.duplicate_selected(false);
    }
    divider(ui);
    if widgets::tool_cell(ui, Icon::Symmetrize, "L → R", false).clicked() {
        editor.symmetrize(Side::Left);
    }
    if widgets::tool_cell(ui, Icon::Mirror, "R → L", false).clicked() {
        editor.symmetrize(Side::Right);
    }
    toggle(ui, Icon::XMirror, "X-Mirror", &mut editor.mirror_edit);
    divider(ui);
    if widgets::tool_cell(ui, Icon::Weights, "Rebind", false).clicked() {
        editor.request_rebind();
    }
}

fn autorig(ui: &mut egui::Ui, editor: &mut Editor) {
    if widgets::tool_cell(ui, Icon::Reset, "Guess", false).clicked() {
        editor.guess_markers();
    }
    divider(ui);
    if widgets::tool_cell(ui, Icon::AutoRig, "Generate", false).clicked() {
        editor.generate_rig();
    }
}

/// Bascules d'affichage et de comportement de la peinture de poids (comme les
/// options du mode Weight Paint de Blender).
fn weights(ui: &mut egui::Ui, editor: &mut Editor) {
    let wp = &mut editor.weight_paint;
    toggle(ui, Icon::XMirror, "X-Mirror", &mut wp.mirror_x);
    toggle(ui, Icon::Normalize, "Normalize", &mut wp.auto_normalize);
    divider(ui);
    toggle(ui, Icon::Grid, "Wire", &mut wp.show_wire);
    toggle(ui, Icon::Eye, "Zero", &mut wp.show_zero_weights);
}

fn toggle(ui: &mut egui::Ui, icon: Icon, label: &str, value: &mut bool) {
    if widgets::tool_cell(ui, icon, label, *value).clicked() {
        *value = !*value;
    }
}
