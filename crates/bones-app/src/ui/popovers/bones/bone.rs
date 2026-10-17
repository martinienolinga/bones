//! Section « Bone » : nom, déformation, liaison au parent et édition
//! symétrique.

use bones_core::Editor;
use bones_core::editor::BoneInfo;
use bones_ui::widgets;

use crate::ui::UiState;
use crate::ui::popovers::note;

pub fn show(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState, info: &BoneInfo) {
    widgets::section_title(ui, "Name");
    let id = egui::Id::unique("bone-rename");
    if !ui.memory(|m| m.has_focus(id)) {
        state.rename.clone_from(&info.name);
    }
    let edit = egui::TextEdit::singleline(&mut state.rename)
        .id(id)
        .font(egui::FontId::proportional(17.0))
        .desired_width(f32::INFINITY)
        .margin(egui::Margin::symmetric(14, 10));
    if ui.add(edit).lost_focus() && state.rename != info.name {
        let name = state.rename.clone();
        editor.rename_selected(&name);
    }

    widgets::section_title(ui, "Behaviour");
    let mut deform = info.deform;
    let mut connected = info.connected;
    let (deform_changed, connected_changed) = widgets::group(ui, |g| {
        let d = g
            .switch_row(
                &mut deform,
                "Deform",
                Some("Moves the mesh (off for helper bones)"),
            )
            .changed();
        let c = g
            .switch_row(
                &mut connected,
                "Connected",
                Some("The head stays on the parent's tail"),
            )
            .changed();
        g.switch_row(
            &mut editor.mirror_edit,
            "X-Mirror",
            Some("Edits also apply to the .l / .r twin"),
        );
        (d, c)
    });
    if deform_changed {
        editor.set_selected_deform(deform);
    }
    if connected_changed && info.parent.is_some() {
        editor.set_bone_connected(info.id, connected);
    }
    if info.parent.is_none() {
        note(ui, "A root bone cannot be connected.");
    }

    widgets::section_title(ui, "Hierarchy");
    let parent = info
        .parent
        .map(|p| editor.rig.skeleton.bone(p).name.clone())
        .unwrap_or_else(|| "None (root)".to_owned());
    widgets::group(ui, |g| {
        g.value_row("Parent", &parent);
        g.value_row("Children", &info.children.to_string());
    });
}
