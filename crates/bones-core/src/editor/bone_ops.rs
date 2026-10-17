//! Édition complète des os : valeurs numériques, hiérarchie, copies,
//! suppression, sélection, et édition symétrique (« X-Axis Mirror »).
//! Messages affichés en anglais.

use glam::Vec3;

use super::Editor;
use crate::rig::mirror::{self, mirror_name};
use crate::{BoneId, side_of};

/// Propriétés numériques regroupées dans une seule étape d'annulation.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Prop {
    Head = 1,
    Tail,
    Length,
    Roll,
}

impl Editor {
    /// Point de reprise unique pour une suite de retouches d'une même valeur.
    fn checkpoint_for(&mut self, bone: BoneId, prop: Prop) {
        let key = (bone, prop as u8);
        if self.coalesce != Some(key) {
            self.checkpoint();
            self.coalesce = Some(key);
        }
    }

    /// Après une retouche : symétrie éventuelle et nouvelle liaison des poids.
    pub(super) fn after_bone_edit(&mut self, bone: BoneId) {
        if self.mirror_edit {
            let s = &self.rig.skeleton;
            if let Some(side) = side_of(&s.bone(bone).name) {
                let mut affected = vec![bone];
                affected.extend(s.bone(bone).parent());
                affected.extend(s.children(bone));
                affected.retain(|b| side_of(&s.bone(*b).name) == Some(side));
                mirror::mirror_bones(&mut self.rig.skeleton, &affected);
            }
        }
        self.rig.touch_skeleton();
    }

    fn counterpart(&self, bone: BoneId) -> Option<BoneId> {
        let s = &self.rig.skeleton;
        mirror_name(&s.bone(bone).name)
            .and_then(|n| s.find(&n))
            .filter(|m| *m != bone)
    }

    // ------------------------------------------------------------------
    // Valeurs numériques (os sélectionné)

    pub fn move_head_to(&mut self, bone: BoneId, p: Vec3) {
        self.checkpoint_for(bone, Prop::Head);
        self.rig.skeleton.set_head_world(bone, p);
        self.after_bone_edit(bone);
    }

    pub fn set_selected_head(&mut self, p: Vec3) {
        if let Some(bone) = self.selected {
            self.move_head_to(bone, p);
        }
    }

    pub fn set_selected_tail(&mut self, p: Vec3) {
        let Some(bone) = self.selected else { return };
        self.checkpoint_for(bone, Prop::Tail);
        self.rig.skeleton.set_tail_world(bone, p);
        self.after_bone_edit(bone);
    }

    pub fn set_selected_length(&mut self, length: f32) {
        let Some(bone) = self.selected else { return };
        self.checkpoint_for(bone, Prop::Length);
        self.rig.skeleton.set_length(bone, length);
        self.after_bone_edit(bone);
    }

    /// Roulis en radians.
    pub fn set_selected_roll(&mut self, roll: f32) {
        let Some(bone) = self.selected else { return };
        self.checkpoint_for(bone, Prop::Roll);
        self.rig.skeleton.set_roll(bone, roll);
        self.after_bone_edit(bone);
    }

    pub fn set_selected_deform(&mut self, deform: bool) {
        let Some(bone) = self.selected else { return };
        self.checkpoint();
        self.rig.skeleton.bone_mut(bone).deform = deform;
        if let Some(m) = self.counterpart(bone).filter(|_| self.mirror_edit) {
            self.rig.skeleton.bone_mut(m).deform = deform;
        }
        self.rig.touch_skeleton();
    }

    pub fn flip_selected(&mut self) {
        let Some(bone) = self.selected else { return };
        self.checkpoint();
        self.rig.skeleton.flip(bone);
        self.after_bone_edit(bone);
    }

    pub fn align_selected(&mut self, axis: Vec3) {
        let Some(bone) = self.selected else { return };
        self.checkpoint();
        self.rig.skeleton.align(bone, axis);
        self.after_bone_edit(bone);
    }

    // ------------------------------------------------------------------
    // Hiérarchie

    /// Change le parent d'un os (`None` : racine) ; avec `connect`, sa tête
    /// rejoint la queue du parent. Retourne `false` si c'est impossible.
    pub fn set_bone_parent(&mut self, child: BoneId, parent: Option<BoneId>, connect: bool) -> bool {
        let s = &self.rig.skeleton;
        let names = (s.bone(child).name.clone(), parent.map(|p| s.bone(p).name.clone()));
        self.checkpoint();
        let selected = self.selected.map(|b| self.rig.skeleton.bone(b).name.clone());
        if let Err(e) = self.rig.set_bone_parent(child, parent, connect) {
            self.drop_checkpoint(true);
            self.status = Some(e.to_string());
            return false;
        }
        if self.mirror_edit {
            self.mirror_reparent(&names.0, names.1.as_deref(), connect);
        }
        self.selected = selected.and_then(|n| self.rig.skeleton.find(&n));
        self.status = Some(match names.1 {
            Some(p) => format!("{} → {p}", names.0),
            None => format!("{} is now a root", names.0),
        });
        true
    }

    /// Même changement de parent pour l'os symétrique, s'il existe.
    fn mirror_reparent(&mut self, child: &str, parent: Option<&str>, connect: bool) {
        let s = &self.rig.skeleton;
        let Some(m) = mirror_name(child).and_then(|n| s.find(&n)) else {
            return;
        };
        let mirrored_parent = match parent {
            None => None,
            // Parent central : le même ; parent latéral : son symétrique.
            Some(p) => match mirror_name(p).and_then(|n| s.find(&n)).or_else(|| s.find(p)) {
                Some(id) => Some(id),
                None => return,
            },
        };
        // Un échec (cycle) laisse simplement le symétrique tel quel.
        let _ = self.rig.set_bone_parent(m, mirrored_parent, connect);
    }

    pub fn set_bone_connected(&mut self, bone: BoneId, connect: bool) {
        self.checkpoint();
        self.rig.skeleton.set_connected(bone, connect);
        if let Some(m) = self.counterpart(bone).filter(|_| self.mirror_edit) {
            self.rig.skeleton.set_connected(m, connect);
        }
        self.after_bone_edit(bone);
    }

    // ------------------------------------------------------------------
    // Copies et suppression

    /// Copie l'os sélectionné (avec ses enfants si demandé), légèrement
    /// décalée vers la droite de l'écran ; la copie est sélectionnée.
    pub fn duplicate_selected(&mut self, with_children: bool) {
        let Some(bone) = self.selected else { return };
        self.checkpoint();
        let copies = self.rig.duplicate_bone(bone, with_children);
        let offset = self.camera.right() * self.rig.skeleton.bone(bone).length * 0.25;
        self.rig.skeleton.offset_world(&copies, offset);
        self.selected = copies.first().copied();
        self.status = Some(format!("Duplicated {} bone(s)", copies.len()));
    }

    /// Coupe l'os sélectionné en `parts` os connectés.
    pub fn subdivide_selected(&mut self, parts: usize) {
        let Some(bone) = self.selected else { return };
        self.checkpoint();
        let (pieces, _) = self.rig.subdivide_bone(bone, parts);
        self.selected = pieces.first().copied();
        self.status = Some(format!("Subdivided into {} bones", pieces.len()));
    }

    pub fn delete_selected_with_children(&mut self) {
        let Some(bone) = self.selected else { return };
        self.checkpoint();
        let count = self.rig.skeleton.subtree(bone).len();
        self.rig.remove_with_children(bone);
        self.selected = None;
        self.status = Some(format!("Deleted {count} bone(s)"));
    }
}
