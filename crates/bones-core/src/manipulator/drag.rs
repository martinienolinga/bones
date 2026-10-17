//! Conversion d'un glissement en transformation. Chaque résultat est
//! relatif au début du geste : l'appelant l'applique à l'état de départ, si
//! bien qu'aucune erreur ne s'accumule.

use std::f32::consts::{PI, TAU};

use glam::{Quat, Vec2, Vec3};

use super::projector::Projector;
use super::{Axis, Frame, ManipDelta, Part};

/// Pas de magnétisme : angle (radians), distance (unités monde), échelle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Snap {
    pub angle: f32,
    pub distance: f32,
    pub scale: f32,
}

/// Facteurs d'échelle admis.
const MIN_SCALE: f32 = 0.01;
const MAX_SCALE: f32 = 100.0;

#[derive(Clone, Copy, Debug)]
pub(super) enum Method {
    /// Point de l'axe le plus proche du rayon sous le doigt.
    AxisRay {
        t0: f32,
    },
    /// Axe presque aligné avec la vue : mouvement écran projeté sur l'axe.
    AxisScreen {
        dir: Vec2,
        px_per_unit: f32,
    },
    Plane {
        normal: Vec3,
        p0: Vec3,
    },
    ViewPlane {
        p0: Vec3,
    },
    /// Angle balayé autour du centre projeté (déroulé au-delà d'un tour).
    Swept {
        axis: Vec3,
        sign: f32,
        last: f32,
        accum: f32,
    },
    /// Anneau vu par la tranche : déplacement le long de sa tangente.
    Tangent {
        axis: Vec3,
        dir: Vec2,
    },
    Trackball,
    ScaleAxis {
        axis: Axis,
        dir: Vec2,
        d0: f32,
    },
    ScaleUniform {
        d0: f32,
    },
}

/// Geste en cours sur une partie du manipulateur.
#[derive(Clone, Copy, Debug)]
pub struct ManipDrag {
    pub part: Part,
    /// Repère au début du geste.
    pub frame: Frame,
    pub(super) projector: Projector,
    pub(super) start: Vec2,
    pub(super) center: Vec2,
    pub(super) radius_px: f32,
    pub(super) method: Method,
    /// Dernier résultat (conservé si un calcul échoue).
    pub last: ManipDelta,
    /// Grandeur affichée : distance, angle (radians) ou facteur.
    pub value: f32,
    pub last_pos: Vec2,
}

impl ManipDrag {
    /// Distance parcourue à l'écran depuis le début du geste.
    pub fn travel(&self) -> f32 {
        self.last_pos.distance(self.start)
    }

    /// Transformation correspondant à la position `pos` du doigt.
    pub fn update(&mut self, pos: Vec2, snap: Option<Snap>) -> ManipDelta {
        self.last_pos = pos;
        let (o, rpx) = (self.frame.origin, self.radius_px);
        let p = self.projector;
        let mut delta = ManipDelta::IDENTITY;
        let mut method = self.method;
        match &mut method {
            Method::AxisRay { .. } | Method::AxisScreen { .. } => {
                let Part::Axis(a) = self.part else {
                    return self.last;
                };
                let d = self.frame.axis(a);
                let mut t = match method {
                    Method::AxisRay { t0 } => axis_param(o, d, &p, pos) - t0,
                    Method::AxisScreen { dir, px_per_unit } if px_per_unit > 1e-6 => {
                        (pos - self.start).dot(dir) / px_per_unit
                    }
                    _ => 0.0,
                };
                if let Some(s) = snap {
                    t = round_to(t, s.distance);
                }
                self.value = t;
                delta.translate = d * t;
            }
            Method::Plane { normal, p0 } => {
                let Some(hit) = p.on_plane(pos, o, *normal) else {
                    return self.last;
                };
                delta.translate = self.snap_vector(hit - *p0, snap);
                self.value = delta.translate.length();
            }
            Method::ViewPlane { p0 } => {
                let Some(hit) = p.on_plane(pos, o, p.forward()) else {
                    return self.last;
                };
                delta.translate = self.snap_vector(hit - *p0, snap);
                self.value = delta.translate.length();
            }
            Method::Swept {
                axis,
                sign,
                last,
                accum,
            } => {
                let a = angle_of(pos - self.center);
                *accum += wrap(a - *last);
                *last = a;
                // Écran y vers le bas : un angle écran croissant tourne dans
                // le sens horaire, soit une rotation négative vue de face.
                let theta = snap_angle(-*accum * *sign, snap);
                self.value = theta;
                delta.rotate = Quat::from_axis_angle(*axis, theta);
            }
            Method::Tangent { axis, dir } => {
                let theta = snap_angle((pos - self.start).dot(*dir) / rpx, snap);
                self.value = theta;
                delta.rotate = Quat::from_axis_angle(*axis, theta);
            }
            Method::Trackball => {
                let d = (pos - self.start) / rpx;
                let (yaw, pitch) = (snap_angle(d.x, snap), snap_angle(d.y, snap));
                delta.rotate = (Quat::from_axis_angle(p.right(), pitch) * Quat::from_axis_angle(p.up(), yaw))
                    .normalize();
                self.value = delta.rotate.to_axis_angle().1;
            }
            Method::ScaleAxis { axis, dir, d0 } => {
                let s = if d0.abs() > 0.25 * rpx {
                    (pos - self.center).dot(*dir) / *d0
                } else {
                    1.0 + (pos - self.start).dot(*dir) / rpx
                };
                let s = snap_scale(s, snap);
                self.value = s;
                delta.scale[axis.index()] = s;
            }
            Method::ScaleUniform { d0 } => {
                let s = if *d0 > 0.25 * rpx {
                    pos.distance(self.center) / *d0
                } else {
                    1.0 + (pos - self.start).dot(Vec2::new(1.0, -1.0).normalize()) / rpx
                };
                let s = snap_scale(s, snap);
                self.value = s;
                delta.scale = Vec3::splat(s);
            }
        }
        self.method = method;
        self.last = delta;
        delta
    }

    /// Arrondit un déplacement au pas, composante par composante dans le
    /// repère du manipulateur.
    fn snap_vector(&self, v: Vec3, snap: Option<Snap>) -> Vec3 {
        let Some(s) = snap else { return v };
        let q = self.frame.orientation;
        let local = q.inverse() * v;
        q * Vec3::new(
            round_to(local.x, s.distance),
            round_to(local.y, s.distance),
            round_to(local.z, s.distance),
        )
    }

    /// Texte affiché près du doigt pendant le geste.
    pub fn readout(&self) -> String {
        match self.part {
            Part::Axis(a) => format!("{} {:+.3} m", a.label(), self.value),
            Part::Plane(_) | Part::Center => format!("{:.3} m", self.value),
            Part::ScaleAxis(_) | Part::ScaleUniform => format!("×{:.2}", self.value),
            Part::Ring(_) | Part::ViewRing | Part::Trackball => {
                format!("{:+.1}°", self.value.to_degrees())
            }
        }
    }
}

/// Paramètre, le long de l'axe (o, d), du point le plus proche du rayon
/// passant sous `pos`.
pub(super) fn axis_param(o: Vec3, d: Vec3, p: &Projector, pos: Vec2) -> f32 {
    let ray = p.ray(pos);
    let r = o - ray.origin;
    let b = d.dot(ray.dir);
    let denom = (1.0 - b * b).max(1e-6);
    (b * ray.dir.dot(r) - d.dot(r)) / denom
}

pub(super) fn angle_of(v: Vec2) -> f32 {
    v.y.atan2(v.x)
}

/// Ramène un angle dans ]−π, π].
fn wrap(a: f32) -> f32 {
    let a = (a + PI).rem_euclid(TAU) - PI;
    if a <= -PI { a + TAU } else { a }
}

fn round_to(v: f32, step: f32) -> f32 {
    if step > 0.0 { (v / step).round() * step } else { v }
}

fn snap_angle(a: f32, snap: Option<Snap>) -> f32 {
    snap.map_or(a, |s| round_to(a, s.angle))
}

fn snap_scale(s: f32, snap: Option<Snap>) -> f32 {
    let s = s.clamp(MIN_SCALE, MAX_SCALE);
    snap.map_or(s, |snap| round_to(s, snap.scale).max(snap.scale))
}
