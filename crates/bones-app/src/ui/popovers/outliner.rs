//! Hiérarchie des os, propriétés de l'os sélectionné et bascules IK/FK des
//! membres (comme le panneau de contrôle d'Auto-Rig Pro).

use bones_core::{BoneId, Editor, LimbMode, Mode, Side, side_of};
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::UiState;

pub fn show(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState) {
    egui::ScrollArea::vertical()
        .id_salt("bones-tree")
        .max_height(280.0)
        .show(ui, |ui| tree(ui, editor));
    ui.add_space(6.0);
    properties(ui, editor, state);
    if !editor.rig.limbs.is_empty() {
        ui.add_space(6.0);
        limbs(ui, editor);
    }
}

/// Ligne d'arbre : guides d'indentation, pastille colorée par côté, nom.
fn tree_row(
    ui: &mut egui::Ui,
    depth: usize,
    name: &str,
    color: egui::Color32,
    selected: bool,
) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 32.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let hover = ui
            .ctx()
            .animate_bool_with_time(response.id.with("h"), response.hovered(), 0.12);
        if selected {
            painter.rect_filled(rect, 10, theme::ACCENT.gamma_multiply(0.32));
        } else if hover > 0.0 {
            painter.rect_filled(rect, 10, egui::Color32::from_white_alpha((12.0 * hover) as u8));
        }
        for level in 0..depth {
            let x = rect.left() + 14.0 + level as f32 * 14.0;
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(1.0, egui::Color32::from_white_alpha(18)),
            );
        }
        let dot = egui::pos2(rect.left() + 14.0 + depth as f32 * 14.0, rect.center().y);
        painter.circle_filled(dot, 4.0, color);
        let font = if selected {
            theme::semibold(14.0)
        } else {
            egui::FontId::proportional(14.0)
        };
        let text = if selected {
            theme::TEXT
        } else {
            theme::mix(theme::TEXT_DIM, theme::TEXT, hover)
        };
        painter.text(
            dot + egui::vec2(12.0, 0.0),
            egui::Align2::LEFT_CENTER,
            name,
            font,
            text,
        );
    }
    response
}

fn tree(ui: &mut egui::Ui, editor: &mut Editor) {
    ui.spacing_mut().item_spacing.y = 2.0;
    let mut clicked = None;
    let skeleton = &editor.rig.skeleton;
    for id in skeleton.ids() {
        let bone = skeleton.bone(id);
        let color = match side_of(&bone.name) {
            Some(Side::Left) => theme::BONE_LEFT,
            Some(Side::Right) => theme::BONE_RIGHT,
            None => theme::BONE_CENTER,
        };
        if tree_row(
            ui,
            skeleton.depth(id),
            &bone.name,
            color,
            editor.selected == Some(id),
        )
        .clicked()
        {
            clicked = Some(id);
        }
    }
    if let Some(id) = clicked {
        editor.select(Some(id));
    }
}

fn properties(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState) {
    widgets::section_title(ui, "Selected bone");
    let Some(bone) = editor.selected else {
        ui.label(egui::RichText::new("Tap a bone to select it.").color(theme::TEXT_FAINT));
        return;
    };
    let b = editor.rig.skeleton.bone(bone);
    let (name, length) = (b.name.clone(), b.length);
    let parent = b.parent().map(|p| editor.rig.skeleton.bone(p).name.clone());
    if state.rename.is_empty() || !ui.memory(|m| m.has_focus(egui::Id::unique("rename"))) {
        state.rename = name;
    }
    let edit = egui::TextEdit::singleline(&mut state.rename)
        .id(egui::Id::unique("rename"))
        .desired_width(f32::INFINITY)
        .margin(egui::Margin::symmetric(12, 10));
    if ui.add(edit).lost_focus() {
        let name = state.rename.clone();
        editor.rename_selected(&name);
    }
    ui.label(
        egui::RichText::new(format!(
            "Parent: {}   ·   Length: {length:.3} m",
            parent.as_deref().unwrap_or("none (root)")
        ))
        .size(13.0)
        .color(theme::TEXT_DIM),
    );
    ui.horizontal(|ui| {
        if editor.mode == Mode::Pose
            && widgets::pill_button(ui, Some(Icon::Reset), "Reset to rest", false).clicked()
        {
            editor.reset_pose(true);
        }
        if editor.mode == Mode::Edit
            && widgets::pill_button(ui, Some(Icon::Delete), "Delete bone", false).clicked()
        {
            editor.delete_selected();
        }
        if widgets::pill_button(ui, None, "Deselect", false).clicked() {
            editor.select(None::<BoneId>);
        }
    });
}

fn limbs(ui: &mut egui::Ui, editor: &mut Editor) {
    widgets::section_title(ui, "Limbs");
    for i in 0..editor.rig.limbs.len() {
        let limb = &editor.rig.limbs[i];
        let mut index = usize::from(limb.mode == LimbMode::Fk);
        let name = limb.name.clone();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(name).size(15.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::segmented(ui, &["IK", "FK"], &mut index).changed() {
                    editor.toggle_limb_mode(i);
                }
            });
        });
    }
}
