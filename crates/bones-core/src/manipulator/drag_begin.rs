//! Début d'un geste : choix de la méthode de conversion selon la partie
//! saisie et l'angle de vue.

use glam::{Vec2, Vec3};

use super::drag::{ManipDrag, Method, angle_of, axis_param};
use super::projector::Projector;
use super::{Frame, ManipDelta, Part};

impl ManipDrag {
    pub fn begin(part: Part, frame: Frame, projector: Projector, pos: Vec2, radius_px: f32) -> Self {
        let o = frame.origin;
        let center = projector.project(o);
        let rw = radius_px * projector.world_per_px(o);
        let screen_dir = |d: Vec3| projector.project(o + d * rw) - center;
        let to_eye = projector.to_eye(o);
        let swept = |axis: Vec3, sign: f32| Method::Swept {
            axis,
            sign,
            last: angle_of(pos - center),
            accum: 0.0,
        };
        let method = match part {
            Part::Axis(a) => {
                let d = frame.axis(a);
                let ray = projector.ray(pos);
                let b = d.dot(ray.dir);
                if 1.0 - b * b > 0.03 {
                    Method::AxisRay {
                        t0: axis_param(o, d, &projector, pos),
                    }
                } else {
                    let s = screen_dir(d);
                    Method::AxisScreen {
                        dir: s.normalize_or_zero(),
                        px_per_unit: s.length() / rw,
                    }
                }
            }
            Part::Plane(a) => {
                let normal = frame.axis(a);
                let p0 = projector.on_plane(pos, o, normal).unwrap_or(o);
                Method::Plane { normal, p0 }
            }
            Part::Center => Method::ViewPlane {
                p0: projector.on_plane(pos, o, projector.forward()).unwrap_or(o),
            },
            Part::Ring(a) => {
                let n = frame.axis(a);
                let facing = n.dot(to_eye);
                if facing.abs() > 0.25 {
                    swept(n, facing.signum())
                } else {
                    // Le point de l'anneau le plus proche de l'observateur
                    // suit le doigt : sa tangente est alors dans l'écran.
                    let front = (to_eye - n * facing).normalize_or(frame.axis(a.others().0));
                    let dir = screen_dir(front + n.cross(front) * 0.1) - screen_dir(front);
                    Method::Tangent {
                        axis: n,
                        dir: dir.normalize_or_zero(),
                    }
                }
            }
            Part::ViewRing => swept(-projector.forward(), 1.0),
            Part::Trackball => Method::Trackball,
            Part::ScaleAxis(axis) => {
                let s = screen_dir(frame.axis(axis));
                let dir = if s.length() > 0.15 * radius_px {
                    s.normalize()
                } else {
                    (pos - center).normalize_or(Vec2::X)
                };
                Method::ScaleAxis {
                    axis,
                    dir,
                    d0: (pos - center).dot(dir),
                }
            }
            Part::ScaleUniform => Method::ScaleUniform {
                d0: pos.distance(center),
            },
        };
        Self {
            part,
            frame,
            projector,
            start: pos,
            center,
            radius_px,
            method,
            last: ManipDelta::IDENTITY,
            value: if part.is_scale() { 1.0 } else { 0.0 },
            last_pos: pos,
        }
    }
}
