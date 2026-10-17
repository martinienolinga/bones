//! Début des gestes : sélection et choix de la manipulation.

use glam::{Vec2, Vec3};

use super::drag::Drag;
use super::{Editor, Handle, Mode, PoseTool};
use crate::math::pick::pick_bone;
use crate::{BoneId, LimbMode, Pin, Viewport, head_tail};

impl Editor {
    pub(super) fn pick(&self, pos: Vec2, vp: &Viewport, radius: f32) -> Option<BoneId> {
        pick_bone(
            &self.rig.skeleton,
            &self.display_world(),
            &self.camera,
            vp,
            pos,
            radius,
        )
    }

    pub(super) fn pick_handle(&self, pos: Vec2, vp: &Viewport, radius: f32) -> Option<(Handle, Vec3)> {
        self.handles()
            .into_iter()
            .filter_map(|(h, p)| {
                let s = self.camera.project(p, vp)?;
                let d = s.distance(pos);
                (d <= radius).then_some((h, p, d))
            })
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(h, p, _)| (h, p))
    }

    pub(super) fn plane_point(&self, pos: Vec2, vp: &Viewport, anchor: Vec3) -> Vec3 {
        self.camera.point_on_view_plane(pos, vp, anchor)
    }

    /// Début de geste à un doigt. Retourne `true` si l'éditeur le prend en
    /// charge ; sinon l'app l'utilise pour la caméra.
    pub fn pointer_down(&mut self, pos: Vec2, vp: &Viewport, radius: f32) -> bool {
        // Une modification en attente est comptée avant que le geste ne la masque.
        self.count_edit(false);
        self.drag_moved = false;
        self.playing = false;
        if self.manipulator_down(pos, vp, radius) {
            return true;
        }
        match self.mode {
            Mode::Pose => self.pose_down(pos, vp, radius),
            Mode::Edit => self.edit_down(pos, vp, radius),
            Mode::AutoRig => self.marker_down(pos, vp, radius),
        }
    }

    fn pose_down(&mut self, pos: Vec2, vp: &Viewport, radius: f32) -> bool {
        if self.pose_tool == PoseTool::Move
            && let Some((handle, p)) = self.pick_handle(pos, vp, radius)
        {
            let (limb, pole) = match handle {
                Handle::IkTarget(i) => (i, false),
                Handle::IkPole(i) => (i, true),
                Handle::Marker(..) => return false,
            };
            self.checkpoint();
            self.snap_limbs();
            let offset = p - self.plane_point(pos, vp, p);
            self.drag = Some(Drag::Limb {
                limb,
                pole,
                anchor: p,
                offset,
            });
            return true;
        }

        let Some(bone) = self.pick(pos, vp, radius) else {
            return false;
        };
        self.selected = Some(bone);
        self.checkpoint();
        self.snap_limbs();
        let world = self.rig.world();
        let skeleton = &self.rig.skeleton;
        let (head, tail) = head_tail(&world[bone.index()], skeleton.bone(bone).length);

        match self.pose_tool {
            PoseTool::Pin => {
                if let Some(i) = self.rig.pins.iter().position(|p| p.bone == bone) {
                    self.rig.pins.remove(i);
                } else {
                    self.rig.pins.push(Pin { bone, target: tail });
                }
                self.store_key_if_any();
            }
            PoseTool::Rotate => {
                let pivot = self.camera.project(head, vp).unwrap_or(pos);
                let d = pos - pivot;
                self.drag = Some(Drag::Rotate {
                    bone,
                    start: self.rig.pose.get(bone),
                    pivot,
                    angle0: d.y.atan2(d.x),
                });
            }
            PoseTool::Move => {
                let limb = self
                    .rig
                    .limbs
                    .iter()
                    .position(|l| l.mode == LimbMode::Ik && l.contains(bone) && l.is_valid(skeleton));
                self.drag = Some(if skeleton.bone(bone).parent().is_none() {
                    Drag::Root {
                        bone,
                        start: self.rig.pose.get(bone),
                        anchor: head,
                        grab: self.plane_point(pos, vp, head),
                    }
                } else if let Some(limb) = limb {
                    let target = self.rig.limbs[limb].target;
                    Drag::Limb {
                        limb,
                        pole: false,
                        anchor: target,
                        offset: target - self.plane_point(pos, vp, target),
                    }
                } else {
                    Drag::Fabrik {
                        chain: skeleton.chain(bone, self.chain_len),
                        anchor: tail,
                        offset: tail - self.plane_point(pos, vp, tail),
                    }
                });
            }
        }
        true
    }

    fn marker_down(&mut self, pos: Vec2, vp: &Viewport, radius: f32) -> bool {
        let Some((Handle::Marker(kind, mirrored), p)) = self.pick_handle(pos, vp, radius) else {
            return false;
        };
        let offset = p - self.plane_point(pos, vp, p);
        self.drag = Some(Drag::Marker {
            kind,
            mirrored,
            anchor: p,
            offset,
        });
        true
    }
}
