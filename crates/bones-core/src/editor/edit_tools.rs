//! Outils du mode Édition : ajuster la queue, ajouter, déplacer la tête,
//! déplacer l'os entier, rouler, changer de parent.

use glam::{Vec2, Vec3};

use super::drag::Drag;
use super::{EditTool, Editor};
use crate::{BoneId, Viewport, head_tail};

/// Manipulations propres au mode Édition.
#[derive(Clone, Debug)]
pub(super) enum EditDrag {
    Head {
        bone: BoneId,
        anchor: Vec3,
        offset: Vec3,
    },
    Bone {
        bone: BoneId,
        start: Vec3,
        grab: Vec3,
    },
    Roll {
        bone: BoneId,
        start: f32,
        pivot: Vec2,
        angle0: f32,
        sign: f32,
    },
}

impl Editor {
    pub(super) fn edit_down(&mut self, pos: Vec2, vp: &Viewport, radius: f32) -> bool {
        let hit = self.pick(pos, vp, radius);
        if self.edit_tool == EditTool::Reparent {
            return self.reparent_tap(hit);
        }
        if self.edit_tool == EditTool::AddBone {
            self.add_bone_down(hit, pos, vp);
            return true;
        }
        let Some(bone) = hit else { return false };
        self.selected = Some(bone);
        self.checkpoint();
        let (head, tail) = self.rig.skeleton.rest_head_tail(bone);
        self.drag = Some(match self.edit_tool {
            EditTool::MoveHead => Drag::Edit(EditDrag::Head {
                bone,
                anchor: head,
                offset: head - self.plane_point(pos, vp, head),
            }),
            EditTool::MoveBone => Drag::Edit(EditDrag::Bone {
                bone,
                start: head,
                grab: self.plane_point(pos, vp, head),
            }),
            EditTool::Roll => {
                let pivot = self.camera.project(head, vp).unwrap_or(pos);
                let d = pos - pivot;
                // Vu depuis la pointe, un roulis positif tourne dans le sens
                // trigonométrique ; à l'écran (y vers le bas), c'est l'inverse.
                let toward_viewer = (tail - head).dot(-self.camera.forward()) > 0.0;
                Drag::Edit(EditDrag::Roll {
                    bone,
                    start: self.rig.skeleton.roll(bone),
                    pivot,
                    angle0: d.y.atan2(d.x),
                    sign: if toward_viewer { -1.0 } else { 1.0 },
                })
            }
            _ => Drag::Tail {
                bone,
                anchor: tail,
                offset: tail - self.plane_point(pos, vp, tail),
            },
        });
        true
    }

    fn add_bone_down(&mut self, hit: Option<BoneId>, pos: Vec2, vp: &Viewport) {
        let world = self.rig.skeleton.rest_world();
        let parent = hit.or(self.selected);
        let head = match parent {
            Some(p) => head_tail(&world[p.index()], self.rig.skeleton.bone(p).length).1,
            None => self.plane_point(pos, vp, self.camera.target),
        };
        self.checkpoint();
        let tail = head + self.camera.up() * self.camera.distance * 0.01;
        let bone = self.rig.add_bone_world("bone", parent, head, tail);
        self.selected = Some(bone);
        self.drag = Some(Drag::NewBone { bone, anchor: head });
    }

    pub(super) fn edit_drag(&mut self, drag: EditDrag, pos: Vec2, vp: &Viewport) {
        let bone = match drag {
            EditDrag::Head { bone, anchor, offset } => {
                let p = self.plane_point(pos, vp, anchor) + offset;
                self.rig.skeleton.set_head_world(bone, p);
                bone
            }
            EditDrag::Bone { bone, start, grab } => {
                let wanted = start + (self.plane_point(pos, vp, start) - grab);
                let delta = wanted - self.rig.skeleton.rest_head_tail(bone).0;
                self.rig.skeleton.translate_world(bone, delta);
                bone
            }
            EditDrag::Roll {
                bone,
                start,
                pivot,
                angle0,
                sign,
            } => {
                let d = pos - pivot;
                self.rig
                    .skeleton
                    .set_roll(bone, start + sign * (d.y.atan2(d.x) - angle0));
                bone
            }
        };
        self.after_bone_edit(bone);
    }

    /// Outil « Parent » : premier toucher = l'enfant, second = son nouveau
    /// parent. Toucher le vide (voir `tap_empty`) en fait une racine.
    fn reparent_tap(&mut self, hit: Option<BoneId>) -> bool {
        let Some(bone) = hit else { return false };
        match self.reparent_child.filter(|c| self.rig.skeleton.contains(*c)) {
            None => {
                self.selected = Some(bone);
                self.reparent_child = Some(bone);
                let name = &self.rig.skeleton.bone(bone).name;
                self.status = Some(format!("{name}: tap the new parent (or empty space for none)"));
            }
            Some(child) if child == bone => self.reparent_child = None,
            Some(child) => {
                self.reparent_child = None;
                self.set_bone_parent(child, Some(bone), false);
            }
        }
        true
    }

    /// Fin de l'outil « Parent » dans le vide : l'enfant devient une racine.
    pub(super) fn reparent_to_root(&mut self) -> bool {
        match self
            .reparent_child
            .take()
            .filter(|c| self.rig.skeleton.contains(*c))
        {
            Some(child) if self.edit_tool == EditTool::Reparent => {
                self.set_bone_parent(child, None, false);
                true
            }
            _ => false,
        }
    }
}
