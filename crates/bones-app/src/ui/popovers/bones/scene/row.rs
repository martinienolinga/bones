//! Dessin et touchers d'une ligne de l'outliner : spécification visuelle,
//! sélection, bascules œil / cadenas, repli, renommage en place et menu
//! contextuel. La structure de l'arbre est dans le module parent.

use bones_core::Editor;
use bones_core::scene::{ObjectId, ObjectRef, SkinMethod};
use bones_ui::icons::Icon;
use bones_ui::theme;
use bones_ui::widgets::{self, RowSpec};

use super::{Key, Node, OutlinerState, Row};
use crate::ui::UiState;
use crate::ui::object_props;

/// Dessine une ligne et applique les touchers.
pub(super) fn render(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState, row: &Row) {
    // Renommage en place d'un objet.
    if let Node::Object(id) = row.node
        && state.outliner.renaming == Some(id)
    {
        rename_row(ui, editor, state, id);
        return;
    }
    let spec = spec_for(editor, row);
    let resp = widgets::outliner_row(ui, spec);
    if let Some(eye) = &resp.eye
        && eye.clicked()
    {
        let item = item_of(row.node);
        let vis = visible_of(editor, row.node);
        let _ = editor.set_object_visible(item, !vis);
    }
    if let Some(lock) = &resp.lock
        && lock.clicked()
    {
        let item = item_of(row.node);
        let locked = locked_of(editor, row.node);
        let _ = editor.set_object_locked(item, !locked);
    }
    if let Some(chev) = &resp.chevron
        && chev.clicked()
    {
        state.outliner.toggle(key_of(row.node));
    }
    if let Node::Object(id) = row.node {
        context_menu(&resp.row, editor, state, id);
        if resp.row.double_clicked() {
            start_rename(editor, state, id);
            return;
        }
    }
    if resp.row.clicked() && !resp.handle_consumed() {
        select(editor, row.node, &mut state.outliner);
    }
}

fn spec_for<'a>(editor: &'a Editor, row: &Row) -> RowSpec<'a> {
    match row.node {
        Node::Body => RowSpec {
            depth: row.depth,
            icon: Icon::Mesh,
            tint: theme::ICON,
            name: &editor.rig.body.name,
            selected: editor.selected_object == Some(ObjectRef::Body),
            has_children: false,
            collapsed: false,
            toggles: true,
            visible: editor.rig.body.visible,
            locked: editor.rig.body.locked,
        },
        Node::Armature => RowSpec {
            depth: row.depth,
            icon: Icon::Hierarchy,
            tint: theme::ICON,
            name: "Armature",
            selected: false,
            has_children: true,
            collapsed: row.collapsed,
            toggles: false,
            visible: true,
            locked: false,
        },
        Node::Bone(id) => {
            let bone = editor.rig.skeleton.bone(id);
            RowSpec {
                depth: row.depth,
                icon: Icon::Bone,
                tint: object_props::bone_tint(&bone.name),
                name: &bone.name,
                selected: editor.selected == Some(id),
                has_children: row.has_children,
                collapsed: row.collapsed,
                toggles: false,
                visible: true,
                locked: false,
            }
        }
        Node::Object(id) => {
            let o = editor.rig.object(id).expect("objet listé");
            RowSpec {
                depth: row.depth,
                icon: if o.binding().is_skinned() {
                    Icon::Mesh
                } else {
                    Icon::Object
                },
                tint: theme::ICON,
                name: &o.name,
                selected: editor.selected_object == Some(ObjectRef::Extra(id)),
                has_children: row.has_children,
                collapsed: row.collapsed,
                toggles: true,
                visible: o.visible,
                locked: o.locked,
            }
        }
    }
}

fn select(editor: &mut Editor, node: Node, st: &mut OutlinerState) {
    st.renaming = None;
    match node {
        Node::Body => editor.select_object(Some(ObjectRef::Body)),
        Node::Object(id) => editor.select_object(Some(ObjectRef::Extra(id))),
        Node::Bone(id) => editor.select(Some(id)),
        Node::Armature => st.toggle(Key::Armature),
    }
}

fn item_of(node: Node) -> ObjectRef {
    match node {
        Node::Object(id) => ObjectRef::Extra(id),
        _ => ObjectRef::Body,
    }
}

fn visible_of(editor: &Editor, node: Node) -> bool {
    match node {
        Node::Object(id) => editor.rig.object(id).is_some_and(|o| o.visible),
        _ => editor.rig.body.visible,
    }
}

fn locked_of(editor: &Editor, node: Node) -> bool {
    match node {
        Node::Object(id) => editor.rig.object(id).is_some_and(|o| o.locked),
        _ => editor.rig.body.locked,
    }
}

fn key_of(node: Node) -> Key {
    match node {
        Node::Bone(id) => Key::Bone(id),
        Node::Object(id) => Key::Object(id),
        _ => Key::Armature,
    }
}

fn start_rename(editor: &mut Editor, state: &mut UiState, id: ObjectId) {
    editor.select_object(Some(ObjectRef::Extra(id)));
    if let Some(o) = editor.rig.object(id) {
        state.rename.clone_from(&o.name);
    }
    state.outliner.renaming = Some(id);
    state.outliner.rename_focus = true;
}

/// Ligne de renommage : champ texte pleine largeur, validé à la perte du focus.
fn rename_row(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState, id: ObjectId) {
    let edit_id = ui.make_persistent_id(("outliner-rename", id.0));
    let edit = egui::TextEdit::singleline(&mut state.rename)
        .id(edit_id)
        .font(egui::FontId::proportional(16.0))
        .desired_width(f32::INFINITY)
        .margin(egui::Margin::symmetric(12, 11));
    let resp = ui.add(edit);
    if std::mem::take(&mut state.outliner.rename_focus) {
        resp.request_focus();
    }
    if resp.lost_focus() {
        let name = state.rename.clone();
        let _ = editor.rename_object(ObjectRef::Extra(id), &name);
        state.outliner.renaming = None;
    }
}

/// Menu contextuel d'un objet (appui long / clic droit).
fn context_menu(resp: &egui::Response, editor: &mut Editor, state: &mut UiState, id: ObjectId) {
    resp.context_menu(|ui| {
        let skinned = editor.rig.object(id).is_some_and(|o| o.binding().is_skinned());
        let bone = editor.selected.is_some();
        if ui.button("Rename").clicked() {
            start_rename(editor, state, id);
            ui.close();
        }
        if ui.button("Duplicate").clicked() {
            let _ = editor.duplicate_object(id, true);
            ui.close();
        }
        if ui.button("Delete").clicked() {
            let _ = editor.delete_object(id);
            ui.close();
        }
        ui.separator();
        if ui
            .add_enabled(bone, egui::Button::new("Parent to selected bone"))
            .clicked()
        {
            let _ = editor.parent_object_to_selected_bone(id);
            ui.close();
        }
        if ui.button("Clear parent").clicked() {
            let _ = editor.clear_object_parent(id, true);
            ui.close();
        }
        ui.separator();
        let label = if skinned { "Make rigid" } else { "Bind to skeleton" };
        if ui.button(label).clicked() {
            let method = (!skinned).then_some(SkinMethod::Transfer);
            let _ = editor.set_object_binding(id, method);
            ui.close();
        }
    });
}
