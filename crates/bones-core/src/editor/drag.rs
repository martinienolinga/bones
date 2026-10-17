//! Manipulations en cours : suivi du doigt, contraintes et fin de geste.

use glam::{Quat, Vec2, Vec3};

use super::edit_tools::EditDrag;
use super::manipulate::ManipGesture;
use super::{Editor, Mode, is_limb_marker, mirror_x};
use crate::ik::{IkSettings, solve_fabrik, solve_pins};
use crate::{BoneId, MarkerKind, Transform, Viewport};

#[derive(Clone, Debug)]
pub(super) enum Drag {
    Fabrik {
        chain: Vec<BoneId>,
        anchor: Vec3,
        offset: Vec3,
    },
    Root {
        bone: BoneId,
        start: Transform,
        anchor: Vec3,
        grab: Vec3,
    },
    Limb {
        limb: usize,
        pole: bool,
        anchor: Vec3,
        offset: Vec3,
    },
    Rotate {
        bone: BoneId,
        start: Transform,
        pivot: Vec2,
        angle0: f32,
    },
    Tail {
        bone: BoneId,
        anchor: Vec3,
        offset: Vec3,
    },
    NewBone {
        bone: BoneId,
        anchor: Vec3,
    },
    Marker {
        kind: MarkerKind,
        mirrored: bool,
        anchor: Vec3,
        offset: Vec3,
    },
    /// Outils de tête, de déplacement et de roulis du mode Édition.
    Edit(EditDrag),
    /// Poignée du manipulateur 3D.
    Manip(Box<ManipGesture>),
}

impl Editor {
    pub fn pointer_move(&mut self, pos: Vec2, vp: &Viewport) {
        // Manipulateur : son état évolue à chaque mouvement (angle déroulé),
        // il est donc repris puis remis en place, sans copie.
        if let Some(Drag::Manip(mut gesture)) = self.drag.take_if(|d| matches!(d, Drag::Manip(_))) {
            self.drag_moved = true;
            self.manipulator_move(&mut gesture, pos);
            self.drag = Some(Drag::Manip(gesture));
            return;
        }
        let Some(drag) = self.drag.clone() else { return };
        self.drag_moved = true;
        match drag {
            Drag::Fabrik {
                chain,
                anchor,
                offset,
            } => {
                let target = self.plane_point(pos, vp, anchor) + offset;
                let skeleton = &self.rig.skeleton;
                solve_fabrik(
                    skeleton,
                    &mut self.rig.pose,
                    &chain,
                    target,
                    IkSettings::default(),
                );
                self.hold_constraints(&chain, None);
            }
            Drag::Root {
                bone,
                start,
                anchor,
                grab,
            } => {
                let moved = self.plane_point(pos, vp, anchor) - grab;
                let rest = self.rig.skeleton.bone(bone).rest;
                let mut delta = start;
                delta.translation += rest.rotation.inverse() * moved / rest.scale;
                self.rig.pose.set(bone, delta);
                self.hold_constraints(&[bone], None);
            }
            Drag::Limb {
                limb,
                pole,
                anchor,
                offset,
            } => {
                let p = self.plane_point(pos, vp, anchor) + offset;
                let l = &mut self.rig.limbs[limb];
                if pole {
                    l.pole = p;
                } else {
                    l.target = p;
                }
                let l = &self.rig.limbs[limb];
                l.solve(&self.rig.skeleton, &mut self.rig.pose);
                let locked = [l.upper, l.lower];
                self.hold_constraints(&locked, Some(limb));
            }
            Drag::Rotate {
                bone,
                start,
                pivot,
                angle0,
            } => {
                let d = pos - pivot;
                let angle = d.y.atan2(d.x) - angle0;
                self.rig.pose.set(bone, start);
                let skeleton = &self.rig.skeleton;
                let parent = skeleton
                    .bone(bone)
                    .parent()
                    .map(|p| self.rig.pose.world(skeleton)[p.index()])
                    .unwrap_or(Transform::IDENTITY);
                let rotation = Quat::from_axis_angle(self.camera.forward(), angle);
                self.rig.pose.rotate_world(skeleton, bone, &parent, rotation);
                let limb = self.rig.limbs.iter().position(|l| l.contains(bone));
                self.hold_constraints(&[bone], limb);
            }
            Drag::Tail { bone, anchor, offset } => {
                let p = self.plane_point(pos, vp, anchor) + offset;
                self.rig.skeleton.set_tail_world(bone, p);
                self.after_bone_edit(bone);
            }
            Drag::Edit(edit) => self.edit_drag(edit, pos, vp),
            Drag::Manip(_) => {}
            Drag::NewBone { bone, anchor } => {
                let p = self.plane_point(pos, vp, anchor);
                self.rig.skeleton.set_tail_world(bone, p);
                self.rig.touch_skeleton();
            }
            Drag::Marker {
                kind,
                mirrored,
                anchor,
                offset,
            } => {
                let mut p = self.plane_point(pos, vp, anchor) + offset;
                if let Some(m) = &mut self.markers {
                    if mirrored {
                        p = mirror_x(p, m.root.x);
                    }
                    // Les marqueurs centraux restent dans le plan de symétrie.
                    if !is_limb_marker(kind) && kind != MarkerKind::Root {
                        p.x = m.root.x;
                    }
                    m.set(kind, p);
                }
            }
        }
    }

    /// Maintient les membres IK et les épingles pendant une manipulation.
    pub(super) fn hold_constraints(&mut self, locked: &[BoneId], driven_limb: Option<usize>) {
        let skeleton = &self.rig.skeleton;
        for (i, limb) in self.rig.limbs.iter().enumerate() {
            if Some(i) == driven_limb || locked.iter().any(|b| limb.contains(*b)) {
                continue;
            }
            limb.solve(skeleton, &mut self.rig.pose);
        }
        if !self.rig.pins.is_empty() {
            let pins = self.rig.pins.clone();
            solve_pins(skeleton, &mut self.rig.pose, &pins, self.chain_len.max(2), locked);
        }
    }

    pub fn pointer_up(&mut self) {
        let Some(drag) = self.drag.take() else { return };
        match drag {
            Drag::NewBone { bone, .. } => {
                // Simple tap : pas d'os de longueur nulle.
                if self.rig.skeleton.bone(bone).length < self.camera.distance * 0.02 {
                    self.drop_checkpoint(true);
                    self.selected = self.selected.filter(|s| self.rig.skeleton.contains(*s));
                }
            }
            // Marqueurs : hors historique, mais enregistrés avec le projet.
            Drag::Marker { .. } => self.count_edit(self.drag_moved),
            Drag::Manip(gesture) => self.manipulator_up(&gesture),
            // Simple tape sur un os (sélection) : rien à annuler.
            _ if !self.drag_moved => self.drop_checkpoint(true),
            Drag::Tail { .. } | Drag::Edit(_) => {}
            _ => {
                self.snap_limbs();
                self.store_key_if_any();
            }
        }
    }

    /// Annule le geste en cours (arrivée d'un second doigt, etc.).
    pub fn pointer_cancel(&mut self) {
        if self.drag.take().is_some() && !matches!(self.mode, Mode::AutoRig) {
            self.drop_checkpoint(true);
        }
    }

    /// Tape dans le vide : désélection (ou, avec l'outil « Parent » en
    /// cours, l'os choisi devient une racine).
    pub fn tap_empty(&mut self) {
        if !self.reparent_to_root() {
            self.selected = None;
        }
    }

    pub fn select(&mut self, bone: Option<BoneId>) {
        self.selected = bone.filter(|b| self.rig.skeleton.contains(*b));
    }
}
