//! Construction des formes de chaque jeu de poignées. Les proportions sont
//! relatives à la longueur des flèches (`1.0`).

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, TAU};

use glam::{Vec2, Vec3};

use super::geometry::{Element, Geometry, Kind, Tint};
use super::projector::Projector;
use super::{Axis, Frame, Part};

/// Segments d'un anneau complet et d'un cercle écran.
const RING_STEPS: usize = 64;
const CIRCLE_STEPS: usize = 48;
const ARC_STEPS: usize = 16;

pub(super) struct Builder<'a> {
    g: &'a mut Geometry,
    p: &'a Projector,
    frame: &'a Frame,
    /// Longueur des flèches en unités monde, puis en pixels.
    rw: f32,
    rpx: f32,
    to_eye: Vec3,
    c: Vec2,
}

impl<'a> Builder<'a> {
    pub fn new(g: &'a mut Geometry, p: &'a Projector, frame: &'a Frame, rw: f32, rpx: f32) -> Self {
        let c = g.center;
        Self {
            g,
            p,
            frame,
            rw,
            rpx,
            to_eye: p.to_eye(frame.origin),
            c,
        }
    }

    pub fn translate(&mut self) {
        self.circle(0.13, Some(Part::Center), Tint::Neutral, Kind::Loop)
            .solid = true;
        self.planes(0.3);
        self.arrows(Part::Axis, true);
    }

    /// Comme la première image de référence : anneaux d'axes, grand anneau
    /// de vue avec sa poignée, axes fins vers la boule centrale.
    pub fn rotate(&mut self) {
        self.circle(1.0, Some(Part::Trackball), Tint::Trackball, Kind::Fill);
        for axis in Axis::ALL {
            self.ring(axis, true);
        }
        self.axis_lines();
        self.circle(1.2, Some(Part::ViewRing), Tint::View, Kind::Loop);
        let handle = self.c + Vec2::new(FRAC_PI_4.cos(), -FRAC_PI_4.sin()) * 1.2 * self.rpx;
        self.disc(handle, 0.06, Some(Part::ViewRing), Tint::Handle);
        for axis in Axis::ALL {
            self.ring(axis, false);
        }
        for axis in Axis::ALL {
            self.ring_arrow(axis);
        }
        self.disc(self.c, 0.085, Some(Part::Trackball), Tint::Ball);
    }

    pub fn scale(&mut self, axis_scale: bool) {
        self.circle(1.12, Some(Part::ScaleUniform), Tint::Neutral, Kind::Loop)
            .width = 0.8;
        if axis_scale {
            self.arrows(Part::ScaleAxis, false);
        } else {
            self.axis_lines();
        }
        self.circle(0.14, Some(Part::ScaleUniform), Tint::Neutral, Kind::Loop)
            .solid = true;
    }

    /// Comme la seconde image de référence : flèches, arcs de rotation et
    /// carrés de plan, dans un fin cercle de vue.
    pub fn combined(&mut self) {
        self.circle(1.12, Some(Part::ViewRing), Tint::View, Kind::Loop)
            .width = 0.7;
        for axis in Axis::ALL {
            self.arc(axis, 0.62);
        }
        self.planes(0.3);
        self.arrows(Part::Axis, true);
        self.disc(self.c, 0.07, Some(Part::Center), Tint::Ball);
    }

    // ------------------------------------------------------------------

    fn world(&self, offset: Vec3) -> Vec2 {
        self.p.project(self.frame.origin + offset * self.rw)
    }

    /// Même projection que [`Self::world`], sans emprunter `self` (pour
    /// remplir les tampons sans allocation intermédiaire).
    fn screen(&self) -> impl Fn(Vec3) -> Vec2 + Copy + 'a {
        let (p, o, rw) = (self.p, self.frame.origin, self.rw);
        move |offset| p.project(o + offset * rw)
    }

    fn circle(&mut self, k: f32, part: Option<Part>, tint: Tint, kind: Kind) -> &mut Element {
        let (c, r) = (self.c, k * self.rpx);
        let points = (0..CIRCLE_STEPS).map(move |i| {
            let t = i as f32 / CIRCLE_STEPS as f32 * TAU;
            c + Vec2::new(t.cos(), t.sin()) * r
        });
        self.g.push(part, tint, kind, points)
    }

    fn disc(&mut self, at: Vec2, k: f32, part: Option<Part>, tint: Tint) {
        let r = k * self.rpx;
        let points = (0..24).map(move |i| {
            let t = i as f32 / 24.0 * TAU;
            at + Vec2::new(t.cos(), t.sin()) * r
        });
        self.g.push(part, tint, Kind::Fill, points);
    }

    /// Flèches (ou tiges à cube pour l'échelle), de la plus lointaine à la
    /// plus proche. Un axe pointé vers l'observateur n'est pas dessiné.
    fn arrows(&mut self, part: fn(Axis) -> Part, head: bool) {
        let mut order = Axis::ALL;
        order.sort_by(|a, b| {
            let da = self.frame.axis(*a).dot(self.to_eye);
            da.total_cmp(&self.frame.axis(*b).dot(self.to_eye))
        });
        for axis in order {
            let d = self.frame.axis(axis);
            let tip = self.world(d);
            let along = tip - self.c;
            if along.length() < 0.2 * self.rpx {
                continue;
            }
            let u = along.normalize();
            let perp = u.perp();
            let (end, tint) = (if head { 0.78 } else { 0.84 }, Tint::Axis(axis));
            let shaft = [self.world(d * 0.2), self.world(d * end)];
            self.g.push(Some(part(axis)), tint, Kind::Line, shaft);
            if head {
                let (base, w) = (shaft[1], 0.085 * self.rpx);
                self.g.push(
                    Some(part(axis)),
                    tint,
                    Kind::Fill,
                    [tip, base + perp * w, base - perp * w],
                );
            } else {
                let (m, h) = (self.world(d * 0.92), 0.065 * self.rpx);
                let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                    .map(|(a, b)| m + (u * a + perp * b) * h);
                self.g.push(Some(part(axis)), tint, Kind::Fill, corners);
            }
        }
    }

    /// Axes fins (décoratifs) du centre vers les anneaux.
    fn axis_lines(&mut self) {
        for axis in Axis::ALL {
            let d = self.frame.axis(axis);
            let line = [self.c, self.world(d)];
            let e = self.g.push(None, Tint::Axis(axis), Kind::Line, line);
            e.width = 0.55;
            e.back = d.dot(self.to_eye) < 0.0;
        }
    }

    /// Carrés de plan, entre les flèches positives de leurs deux axes.
    fn planes(&mut self, offset: f32) {
        for axis in Axis::ALL {
            if self.frame.axis(axis).dot(self.to_eye).abs() < 0.2 {
                continue; // plan vu par la tranche
            }
            let (a, b) = axis.others();
            let (u, v) = (self.frame.axis(a), self.frame.axis(b));
            let (q, h) = ((u + v) * offset, 0.075);
            let corners = [(-h, -h), (h, -h), (h, h), (-h, h)].map(|(x, y)| self.world(q + u * x + v * y));
            self.g
                .push(Some(Part::Plane(axis)), Tint::Axis(axis), Kind::Fill, corners);
        }
    }

    /// Anneau complet découpé en moitiés avant et arrière ; seules celles
    /// de la passe demandée sont ajoutées.
    fn ring(&mut self, axis: Axis, back_pass: bool) {
        let (a, b) = axis.others();
        let (u, v) = (self.frame.axis(a), self.frame.axis(b));
        let dir = |k: usize| {
            let t = k as f32 / RING_STEPS as f32 * TAU;
            u * t.cos() + v * t.sin()
        };
        let mut front = [false; RING_STEPS];
        for (k, f) in front.iter_mut().enumerate() {
            *f = dir(k).dot(self.to_eye) >= 0.0;
        }
        let (part, screen) = (Some(Part::Ring(axis)), self.screen());
        let Some(s) = (0..RING_STEPS).find(|&k| front[k] != front[(k + RING_STEPS - 1) % RING_STEPS]) else {
            if front[0] != back_pass {
                let points = (0..RING_STEPS).map(move |k| screen(dir(k)));
                self.g.push(part, Tint::Axis(axis), Kind::Loop, points).back = back_pass;
            }
            return;
        };
        let mut i = 0;
        while i < RING_STEPS {
            let flag = front[(s + i) % RING_STEPS];
            let mut j = i;
            while j < RING_STEPS && front[(s + j) % RING_STEPS] == flag {
                j += 1;
            }
            if flag != back_pass {
                let points = (i..=j).map(move |k| screen(dir((s + k) % RING_STEPS)));
                self.g.push(part, Tint::Axis(axis), Kind::Line, points).back = back_pass;
            }
            i = j;
        }
    }

    /// Petit triangle sur l'anneau indiquant le sens de rotation positif,
    /// à mi-chemin entre le bord apparent et le point le plus proche.
    fn ring_arrow(&mut self, axis: Axis) {
        let n = self.frame.axis(axis);
        let front = self.to_eye - n * n.dot(self.to_eye);
        let m = match (n.cross(self.to_eye).try_normalize(), front.try_normalize()) {
            (Some(major), Some(front)) => (major + front).normalize(),
            _ => self.frame.axis(axis.others().0),
        };
        let at = self.world(m);
        let Some(dir) = (self.world(m + n.cross(m) * 0.1) - at).try_normalize() else {
            return;
        };
        let (tip, base, w) = (
            at + dir * 0.09 * self.rpx,
            at - dir * 0.03 * self.rpx,
            0.055 * self.rpx,
        );
        let tri = [tip, base + dir.perp() * w, base - dir.perp() * w];
        self.g
            .push(Some(Part::Ring(axis)), Tint::Axis(axis), Kind::Fill, tri);
    }

    /// Quart d'anneau entre les flèches positives des deux autres axes.
    fn arc(&mut self, axis: Axis, k: f32) {
        let (a, b) = axis.others();
        let (u, v) = (self.frame.axis(a), self.frame.axis(b));
        let (margin, screen) = (0.1, self.screen());
        let points = (0..=ARC_STEPS).map(move |i| {
            let t = margin + i as f32 / ARC_STEPS as f32 * (FRAC_PI_2 - 2.0 * margin);
            screen((u * t.cos() + v * t.sin()) * k)
        });
        self.g
            .push(Some(Part::Ring(axis)), Tint::Axis(axis), Kind::Line, points);
    }
}
