//! Animation FBX : pile → calque → nœuds de courbes (`Lcl Translation`,
//! `Lcl Rotation`, `Lcl Scaling`) → courbes par axe. Les courbes sont
//! interpolées linéairement puis échantillonnées à la cadence du fichier.

use std::collections::HashMap;

use fbxcel::low::v7400::AttributeValue;
use glam::DVec3;

use super::document::{Document, Props, child_attr};
use super::models::Models;
use super::skin::Rigging;
use crate::Clip;
use crate::io::rigging::BuiltSkeleton;

/// Unités de temps FBX par seconde.
const TICKS: f64 = 46_186_158_000.0;

struct Curve {
    times: Vec<i64>,
    values: Vec<f64>,
}

impl Curve {
    fn at(&self, t: i64) -> f64 {
        let n = self.times.len().min(self.values.len());
        if n == 0 {
            return 0.0;
        }
        if t <= self.times[0] {
            return self.values[0];
        }
        if t >= self.times[n - 1] {
            return self.values[n - 1];
        }
        let i = self.times[..n].partition_point(|x| *x <= t);
        let (t0, t1) = (self.times[i - 1], self.times[i]);
        let u = (t - t0) as f64 / (t1 - t0).max(1) as f64;
        self.values[i - 1] + (self.values[i] - self.values[i - 1]) * u
    }
}

/// Canal animé d'un modèle : une courbe ou une valeur fixe par axe.
struct Channel {
    axes: [Option<Curve>; 3],
    defaults: DVec3,
}

impl Channel {
    fn at(&self, t: i64) -> DVec3 {
        let v = |i: usize| self.axes[i].as_ref().map(|c| c.at(t)).unwrap_or(self.defaults[i]);
        DVec3::new(v(0), v(1), v(2))
    }
}

#[derive(Default)]
struct Tracks {
    translation: Option<Channel>,
    rotation: Option<Channel>,
    scaling: Option<Channel>,
}

fn curve(o: &super::document::Object) -> Option<Curve> {
    let times = child_attr(&o.node, "KeyTime")?.get_arr_i64()?.to_vec();
    let values = match child_attr(&o.node, "KeyValueFloat")? {
        AttributeValue::ArrF32(v) => v.iter().map(|x| *x as f64).collect(),
        AttributeValue::ArrF64(v) => v.to_vec(),
        _ => return None,
    };
    Some(Curve { times, values })
}

/// Animation de la première pile (import d'un modèle).
pub(super) fn sample(
    doc: &Document,
    models: &Models,
    rigging: &Rigging,
    axis: glam::DMat4,
    fps: f32,
    built: &BuiltSkeleton,
) -> Option<Clip> {
    let mut stacks: Vec<_> = doc.of_class("AnimationStack").collect();
    stacks.sort_by_key(|s| s.id);
    sample_stack(doc, models, rigging, axis, fps, built, stacks.first()?)
}

/// Toutes les piles d'animation (prises Maya, clips Mixamo…) avec leur nom.
pub(super) fn sample_all(
    doc: &Document,
    models: &Models,
    rigging: &Rigging,
    axis: glam::DMat4,
    fps: f32,
    built: &BuiltSkeleton,
) -> Vec<(String, Clip)> {
    let mut stacks: Vec<_> = doc.of_class("AnimationStack").collect();
    stacks.sort_by_key(|s| s.id);
    stacks
        .into_iter()
        .filter_map(|stack| {
            let name = stack.name.rsplit("::").next().unwrap_or(&stack.name).to_owned();
            sample_stack(doc, models, rigging, axis, fps, built, stack).map(|c| (name, c))
        })
        .collect()
}

fn sample_stack(
    doc: &Document,
    models: &Models,
    rigging: &Rigging,
    axis: glam::DMat4,
    fps: f32,
    built: &BuiltSkeleton,
    stack: &super::document::Object,
) -> Option<Clip> {
    let layer = doc.children_of(stack.id, "AnimationLayer").into_iter().next()?;

    let mut tracks: HashMap<i64, Tracks> = HashMap::new();
    let (mut first, mut last) = (i64::MAX, i64::MIN);
    for node in doc.children_of(layer.id, "AnimationCurveNode") {
        let Some(target) = doc
            .parents
            .get(&node.id)
            .into_iter()
            .flatten()
            .find(|l| l.property.is_some())
        else {
            continue;
        };
        let props = Props::of(&node.node);
        let axes = ["d|X", "d|Y", "d|Z"].map(|p| {
            let c = doc.children_with_property(node.id, p).into_iter().find_map(curve);
            if let Some(c) = &c {
                first = first.min(*c.times.first().unwrap_or(&first));
                last = last.max(*c.times.last().unwrap_or(&last));
            }
            c
        });
        let defaults = DVec3::new(
            props.f64("d|X").unwrap_or(0.0),
            props.f64("d|Y").unwrap_or(0.0),
            props.f64("d|Z").unwrap_or(0.0),
        );
        let channel = Channel { axes, defaults };
        let entry = tracks.entry(target.id).or_default();
        match target.property.as_deref() {
            Some("Lcl Translation") => entry.translation = Some(channel),
            Some("Lcl Rotation") => entry.rotation = Some(channel),
            Some("Lcl Scaling") => entry.scaling = Some(channel),
            _ => {}
        }
    }
    if tracks.is_empty() || first > last {
        return None;
    }
    let stack_props = Props::of(&stack.node);
    let start = stack_props
        .i64("LocalStart")
        .filter(|s| *s <= first)
        .unwrap_or(first);
    let stop = stack_props
        .i64("LocalStop")
        .filter(|s| *s > start)
        .unwrap_or(last)
        .max(last);
    let duration = (stop - start) as f64 / TICKS;
    let frames = ((duration * fps as f64).round() as u32 + 1).max(2);

    Some(built.clip(fps, frames, |f| {
        let t = start + (f as f64 / fps as f64 * TICKS) as i64;
        let globals = models.globals(|id, m| {
            let p = &m.parts;
            match tracks.get(&id) {
                Some(tr) => p.matrix(
                    tr.translation.as_ref().map(|c| c.at(t)).unwrap_or(p.translation),
                    tr.rotation.as_ref().map(|c| c.at(t)).unwrap_or(p.rotation),
                    tr.scaling.as_ref().map(|c| c.at(t)).unwrap_or(p.scaling),
                ),
                None => p.rest(),
            }
        });
        rigging
            .ids
            .iter()
            .map(|id| (axis * globals[id]).as_mat4())
            .collect()
    }))
}
