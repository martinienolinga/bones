//! Ce que le rendu et l'interface lisent des objets : appels de dessin
//! (maillage, matrice, poids, révisions) et lignes de la liste des objets.

use std::sync::Arc;

use glam::Mat4;

use super::{Binding, ObjectId, ObjectRef, Parent};
use crate::{Mesh, Rig, Skin, Transform};

/// Poids d'un objet déformé, pour le rendu.
#[derive(Clone, Debug)]
pub struct SkinDraw {
    pub skin: Arc<Skin>,
    /// Révision des poids (clé du tampon GPU des poids).
    pub revision: u64,
}

/// Tout ce qu'il faut pour dessiner un objet visible.
#[derive(Clone, Debug)]
pub struct ObjectDraw {
    pub id: ObjectId,
    pub mesh: Arc<Mesh>,
    /// Révision du maillage (clé des tampons GPU, avec `id`).
    pub mesh_revision: u64,
    /// Objet rigide : matrice monde. Objet déformé : placement de liaison,
    /// appliqué avant le skinning par la palette d'os du corps.
    pub model: Mat4,
    /// `Some` pour un objet déformé par le squelette.
    pub skin: Option<SkinDraw>,
    pub selected: bool,
}

/// Une ligne de la liste des objets (corps en premier, puis les objets en
/// profondeur d'abord, dans l'ordre de la liste).
#[derive(Clone, Debug, PartialEq)]
pub struct OutlinerRow {
    pub item: ObjectRef,
    pub name: String,
    /// Profondeur dans la hiérarchie des objets (0 : attaché au monde ou à un os).
    pub depth: usize,
    pub parent: Parent,
    pub skinned: bool,
    pub visible: bool,
    pub locked: bool,
    /// Nombre d'enfants objets directs.
    pub children: usize,
}

impl Rig {
    /// Appels de dessin des objets visibles. `bone_world` : os tels
    /// qu'affichés (posés ou au repos), `rest_world` : os au repos.
    pub fn object_draws(
        &self,
        bone_world: &[Transform],
        rest_world: &[Transform],
        selected: Option<ObjectId>,
    ) -> Vec<ObjectDraw> {
        if self.objects.is_empty() {
            return Vec::new();
        }
        let worlds = self.object_worlds(bone_world);
        let skinned = self.objects.iter().any(|o| o.binding.is_skinned());
        let placements = if skinned {
            self.object_worlds(rest_world)
        } else {
            Vec::new()
        };
        self.objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.visible)
            .map(|(i, o)| {
                let (model, skin) = match &o.binding {
                    Binding::Rigid => (worlds[i].to_matrix(), None),
                    Binding::Skinned(s) => (
                        placements[i].to_matrix(),
                        Some(SkinDraw {
                            skin: s.skin.clone(),
                            revision: o.revisions.skin,
                        }),
                    ),
                };
                ObjectDraw {
                    id: o.id,
                    mesh: o.mesh.clone(),
                    mesh_revision: o.revisions.mesh,
                    model,
                    skin,
                    selected: selected == Some(o.id),
                }
            })
            .collect()
    }

    /// Lignes de la liste des objets.
    pub fn outliner_rows(&self) -> Vec<OutlinerRow> {
        let mut rows = vec![OutlinerRow {
            item: ObjectRef::Body,
            name: self.body.name.clone(),
            depth: 0,
            parent: Parent::World,
            skinned: !self.skeleton.is_empty(),
            visible: self.body.visible,
            locked: self.body.locked,
            children: 0,
        }];
        let mut seen = vec![false; self.objects.len()];
        let is_root = |p: Parent| match p {
            Parent::Object(id) => self.object(id).is_none(),
            _ => true,
        };
        // Racines d'abord, puis tout objet resté à l'écart (document incohérent).
        let roots = (0..self.objects.len()).filter(|i| is_root(self.objects[*i].parent));
        let order: Vec<usize> = roots.chain(0..self.objects.len()).collect();
        for start in order {
            let mut stack = vec![(start, 0usize)];
            while let Some((i, depth)) = stack.pop() {
                if seen[i] {
                    continue;
                }
                seen[i] = true;
                let o = &self.objects[i];
                let children: Vec<usize> = (0..self.objects.len())
                    .filter(|c| self.objects[*c].parent == Parent::Object(o.id))
                    .collect();
                rows.push(OutlinerRow {
                    item: ObjectRef::Extra(o.id),
                    name: o.name.clone(),
                    depth,
                    parent: o.parent,
                    skinned: o.binding.is_skinned(),
                    visible: o.visible,
                    locked: o.locked,
                    children: children.len(),
                });
                stack.extend(children.into_iter().rev().map(|c| (c, depth + 1)));
            }
        }
        rows
    }
}
