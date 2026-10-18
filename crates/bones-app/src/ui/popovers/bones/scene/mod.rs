//! Arbre de scène unifié (outliner), à la Procreate / Blender : le corps, le
//! squelette (nœud « Armature » repliable, os indentés) et les objets
//! supplémentaires, imbriqués sous leur parent (os ou objet). Chaque ligne
//! porte un œil (visibilité) et un cadenas (verrou) ; un toucher sélectionne,
//! un double-toucher renomme, un appui long ouvre le menu contextuel.
//!
//! Le repli vit dans l'interface (`OutlinerState`), pas dans le cœur. Le
//! dessin et les touchers d'une ligne sont dans [`row`].

mod row;

use std::collections::HashSet;

use bones_core::scene::{ObjectId, ObjectRef, Parent};
use bones_core::{BoneId, Editor};
use bones_ui::icons::{self, Icon};
use bones_ui::theme;

use crate::ui::UiState;
use crate::ui::object_props;
use crate::ui::popovers::note;

/// Clé de repli d'une branche de l'arbre.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Armature,
    Bone(BoneId),
    Object(ObjectId),
}

/// État de l'outliner (non enregistré) : branches repliées, recherche et
/// objet en cours de renommage.
#[derive(Default)]
pub struct OutlinerState {
    collapsed: HashSet<Key>,
    pub search: String,
    renaming: Option<ObjectId>,
    /// Demande de focus au premier affichage du champ de renommage.
    rename_focus: bool,
}

impl OutlinerState {
    fn is_open(&self, key: Key) -> bool {
        !self.collapsed.contains(&key)
    }

    fn toggle(&mut self, key: Key) {
        if !self.collapsed.remove(&key) {
            self.collapsed.insert(key);
        }
    }
}

/// Nœud affiché dans l'arbre.
#[derive(Clone, Copy)]
enum Node {
    Body,
    Armature,
    Bone(BoneId),
    Object(ObjectId),
}

struct Row {
    node: Node,
    depth: usize,
    has_children: bool,
    collapsed: bool,
}

/// Recherche puis arbre de scène.
pub fn tree(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState) {
    search_field(ui, &mut state.outliner.search);
    ui.add_space(6.0);
    let rows = build(editor, &state.outliner);
    egui::Frame::new()
        .fill(theme::WIDGET)
        .corner_radius(egui::CornerRadius::same(12))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.set_width(ui.available_width());
            for r in rows {
                row::render(ui, editor, state, &r);
            }
        });
}

/// Panneau de propriétés sous l'arbre, selon la sélection.
pub fn selection_props(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState) {
    match editor.selected_object {
        Some(ObjectRef::Extra(id)) => object_props::props(ui, editor, state, id, "outliner-obj"),
        Some(ObjectRef::Body) => note(
            ui,
            "The body is the main mesh: it cannot be deleted or reparented.",
        ),
        None => note(ui, "Tap a row to select it. Add objects in Object mode."),
    }
}

/// Construit les lignes affichées (corps, armature, objets), en respectant le
/// repli ; une recherche aplatit l'arbre en liste filtrée.
fn build(editor: &Editor, st: &OutlinerState) -> Vec<Row> {
    let rig = &editor.rig;
    let query = st.search.trim().to_lowercase();
    if !query.is_empty() {
        return search_rows(editor, &query);
    }
    let mut rows = vec![Row {
        node: Node::Body,
        depth: 0,
        has_children: false,
        collapsed: false,
    }];
    if !rig.skeleton.is_empty() {
        let open = st.is_open(Key::Armature);
        rows.push(Row {
            node: Node::Armature,
            depth: 0,
            has_children: true,
            collapsed: !open,
        });
        if open {
            for id in rig.skeleton.ids() {
                let depth = 1 + rig.skeleton.depth(id);
                let objs: Vec<ObjectId> = rig.object_children(Parent::Bone(id)).collect();
                let bopen = st.is_open(Key::Bone(id));
                rows.push(Row {
                    node: Node::Bone(id),
                    depth,
                    has_children: !objs.is_empty(),
                    collapsed: !bopen,
                });
                if bopen {
                    for o in objs {
                        emit_object(&mut rows, editor, st, o, depth + 1);
                    }
                }
            }
        }
    }
    for o in rig.objects() {
        let root = match o.parent() {
            Parent::World => true,
            Parent::Object(p) => rig.object(p).is_none(),
            Parent::Bone(_) => false,
        };
        if root {
            emit_object(&mut rows, editor, st, o.id(), 0);
        }
    }
    rows
}

/// Liste plate filtrée par nom (corps, os, objets).
fn search_rows(editor: &Editor, query: &str) -> Vec<Row> {
    let rig = &editor.rig;
    let mut rows = Vec::new();
    let flat = |node: Node| Row {
        node,
        depth: 0,
        has_children: false,
        collapsed: false,
    };
    if rig.body.name.to_lowercase().contains(query) {
        rows.push(flat(Node::Body));
    }
    for id in rig.skeleton.ids() {
        if rig.skeleton.bone(id).name.to_lowercase().contains(query) {
            rows.push(flat(Node::Bone(id)));
        }
    }
    for o in rig.objects() {
        if o.name.to_lowercase().contains(query) {
            rows.push(flat(Node::Object(o.id())));
        }
    }
    rows
}

fn emit_object(rows: &mut Vec<Row>, editor: &Editor, st: &OutlinerState, id: ObjectId, depth: usize) {
    let children = editor.object_children_of(ObjectRef::Extra(id));
    let open = st.is_open(Key::Object(id));
    rows.push(Row {
        node: Node::Object(id),
        depth,
        has_children: !children.is_empty(),
        collapsed: !open,
    });
    if open {
        for c in children {
            emit_object(rows, editor, st, c, depth + 1);
        }
    }
}

/// Champ de recherche : pastille arrondie avec glyphe.
fn search_field(ui: &mut egui::Ui, text: &mut String) {
    egui::Frame::new()
        .fill(theme::WIDGET)
        .corner_radius(egui::CornerRadius::same(16))
        .inner_margin(egui::Margin::symmetric(12, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (glyph, _) = ui.allocate_exact_size(egui::vec2(18.0, 20.0), egui::Sense::hover());
                icons::paint(ui.painter(), glyph, Icon::Select, theme::ICON_DIM);
                let edit = egui::TextEdit::singleline(text)
                    .hint_text("Search")
                    .frame(egui::Frame::NONE)
                    .desired_width(f32::INFINITY);
                ui.add(edit);
            });
        });
}
