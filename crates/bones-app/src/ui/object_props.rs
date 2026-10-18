//! Propriétés de l'objet sélectionné, partagées par l'outliner (studio des
//! os) et par le panneau du mode Objet : nom, parent, liaison (rigide /
//! déformé) et transformation (position, rotation, échelle). Toutes les
//! retouches passent par l'API de l'éditeur, donc par l'historique.

use bones_core::editor::ObjectInfo;
use bones_core::scene::{ObjectId, ObjectRef, Parent, SkinMethod};
use bones_core::{Editor, side_of};
use bones_ui::{theme, widgets};
use glam::{EulerRot, Quat, Vec3};

use super::UiState;

const AXES: [(&str, egui::Color32); 3] = [
    ("X", egui::Color32::from_rgb(0xff, 0x45, 0x3a)),
    ("Y", egui::Color32::from_rgb(0x30, 0xd1, 0x58)),
    ("Z", egui::Color32::from_rgb(0x0a, 0x84, 0xff)),
];

/// Propriétés de l'objet `id`. `salt` isole les identifiants egui quand le
/// panneau est affiché à deux endroits à la fois.
pub fn props(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState, id: ObjectId, salt: &str) {
    let Some(info) = editor.object_info(id) else {
        widgets::note(ui, "Select an object to edit it.");
        return;
    };
    ui.push_id(salt, |ui| {
        name_field(ui, editor, state, &info);
        parent_section(ui, editor, &info);
        binding_section(ui, editor, &info);
        transform_section(ui, editor, &info);
        actions(ui, editor, id, info.locked);
    });
}

/// Champ de renommage en place (validé à la perte du focus).
fn name_field(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState, info: &ObjectInfo) {
    widgets::section_title(ui, "Name");
    let edit_id = ui.make_persistent_id("object-rename");
    if !ui.memory(|m| m.has_focus(edit_id)) {
        state.rename.clone_from(&info.name);
    }
    let edit = egui::TextEdit::singleline(&mut state.rename)
        .id(edit_id)
        .font(egui::FontId::proportional(17.0))
        .desired_width(f32::INFINITY)
        .margin(egui::Margin::symmetric(14, 10));
    if ui.add(edit).lost_focus() && state.rename.trim() != info.name {
        let name = state.rename.clone();
        let _ = editor.rename_object(ObjectRef::Extra(info.id), &name);
    }
}

/// Parent : étiquette courante et boutons (os sélectionné, détacher).
fn parent_section(ui: &mut egui::Ui, editor: &mut Editor, info: &ObjectInfo) {
    widgets::section_title(ui, "Parent");
    let label = parent_label(editor, info.parent);
    widgets::group(ui, |g| {
        g.value_row("Parent", &label);
    });
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.spacing_mut().item_spacing.x = 6.0;
        let bone = editor.selected;
        let to_bone = ui.add_enabled_ui(bone.is_some(), |ui| {
            widgets::chip_button(ui, "To selected bone", false)
        });
        if to_bone.inner.clicked() {
            let _ = editor.parent_object_to_selected_bone(info.id);
        }
        if !matches!(info.parent, Parent::World) && widgets::chip_button(ui, "Clear parent", false).clicked()
        {
            let _ = editor.clear_object_parent(info.id, true);
        }
    });
    if editor.selected.is_none() {
        widgets::note(
            ui,
            "Select a bone to parent this object to it (it will follow the pose).",
        );
    }
}

/// Nom lisible d'un parent.
fn parent_label(editor: &Editor, parent: Parent) -> String {
    match parent {
        Parent::World => "World".to_owned(),
        Parent::Bone(b) => format!("Bone · {}", editor.rig.skeleton.bone(b).name),
        Parent::Object(o) => editor
            .rig
            .object(o)
            .map(|x| x.name.clone())
            .unwrap_or_else(|| "World".to_owned()),
    }
}

/// Liaison : rigide (suit le parent) ou déformé par le squelette.
fn binding_section(ui: &mut egui::Ui, editor: &mut Editor, info: &ObjectInfo) {
    widgets::section_title(ui, "Binding");
    let mut index = usize::from(info.skin_method.is_some());
    if widgets::segmented(ui, &["Rigid", "Skinned"], &mut index).changed() {
        let method = (index == 1).then_some(SkinMethod::Transfer);
        let _ = editor.set_object_binding(info.id, method);
    }
    widgets::note(
        ui,
        if info.binding_pending {
            "Computing skin weights…"
        } else if index == 1 {
            "Deformed by the skeleton (weights transferred from the body)."
        } else {
            "Rigidly follows its parent (prop)."
        },
    );
}

/// Transformation locale : position, rotation (degrés) et échelle.
fn transform_section(ui: &mut egui::Ui, editor: &mut Editor, info: &ObjectInfo) {
    let mut local = info.local;
    let mut translation = local.translation;
    let (ex, ey, ez) = local.rotation.to_euler(EulerRot::XYZ);
    let mut euler = Vec3::new(ex.to_degrees(), ey.to_degrees(), ez.to_degrees());
    let mut scale = local.scale;

    widgets::section_title(ui, "Position (m)");
    let pos = widgets::group(ui, |g| {
        g.custom(50.0, |ui| {
            vec3_row(ui, "Pos", &mut translation, -100.0..=100.0, 0.002, 3)
        })
    });
    widgets::section_title(ui, "Rotation (°)");
    let rot = widgets::group(ui, |g| {
        g.custom(50.0, |ui| vec3_row(ui, "Rot", &mut euler, -180.0..=180.0, 0.5, 1))
    });
    widgets::section_title(ui, "Scale");
    let scl = widgets::group(ui, |g| {
        g.custom(50.0, |ui| {
            vec3_row(ui, "Scale", &mut scale, 0.01..=100.0, 0.005, 3)
        })
    });

    if pos || rot || scl {
        local.translation = translation;
        local.rotation = Quat::from_euler(
            EulerRot::XYZ,
            euler.x.to_radians(),
            euler.y.to_radians(),
            euler.z.to_radians(),
        );
        local.scale = scale.max(Vec3::splat(0.001));
        let _ = editor.set_object_transform(info.id, local);
    }
}

/// Dupliquer et supprimer l'objet.
fn actions(ui: &mut egui::Ui, editor: &mut Editor, id: ObjectId, locked: bool) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        ui.spacing_mut().item_spacing.x = 8.0;
        if widgets::pill_button(ui, Some(bones_ui::icons::Icon::Duplicate), "Duplicate", false).clicked() {
            let _ = editor.duplicate_object(id, true);
        }
        let del = ui.add_enabled_ui(!locked, |ui| {
            widgets::pill_button(ui, Some(bones_ui::icons::Icon::Delete), "Delete", false)
        });
        if del.inner.clicked() {
            let _ = editor.delete_object(id);
        }
    });
}

/// Libellé puis trois pastilles X / Y / Z ; vrai si une valeur a changé.
fn vec3_row(
    ui: &mut egui::Ui,
    label: &str,
    v: &mut Vec3,
    range: std::ops::RangeInclusive<f32>,
    speed: f64,
    decimals: usize,
) -> bool {
    ui.add_sized(
        [52.0, 30.0],
        egui::Label::new(egui::RichText::new(label).size(16.0).color(theme::TEXT_LIST)),
    );
    let width = ((ui.available_width() - 3.0 * 22.0) / 3.0 - 6.0).max(52.0);
    let mut changed = false;
    for (i, (name, color)) in AXES.iter().enumerate() {
        ui.label(
            egui::RichText::new(*name)
                .font(theme::semibold(13.0))
                .color(*color),
        );
        changed |= widgets::drag_chip(ui, &mut v[i], range.clone(), speed, decimals, "", width).changed();
    }
    changed
}

/// Couleur de côté d'un os (pour les lignes d'os de l'outliner).
pub fn bone_tint(name: &str) -> egui::Color32 {
    match side_of(name) {
        Some(bones_core::Side::Left) => theme::BONE_LEFT,
        Some(bones_core::Side::Right) => theme::BONE_RIGHT,
        None => theme::BONE_CENTER,
    }
}
