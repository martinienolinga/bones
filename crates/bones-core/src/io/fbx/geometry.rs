//! Géométrie FBX : sommets de contrôle, polygones et couches (normales, UV,
//! matériaux), selon tous les modes de correspondance usuels.

use std::collections::HashMap;
use std::ops::Range;

use fbxcel::low::v7400::AttributeValue;
use fbxcel::tree::v7400::NodeHandle;
use glam::{Vec2, Vec3};

use super::document::{Object, child_attr};
use crate::Mesh;

/// Maillage d'un objet `Geometry`, dans son propre repère.
pub(super) struct GeometryPart {
    pub mesh: Mesh,
    /// Sommet de contrôle d'origine de chaque sommet (pour les poids).
    pub control_point: Vec<u32>,
    /// Nombre de sommets de contrôle de la géométrie.
    pub control_points: usize,
    /// Plage d'indices et emplacement de matériau du modèle.
    pub slots: Vec<(Range<u32>, usize)>,
}

fn floats(a: &AttributeValue) -> Option<Vec<f64>> {
    match a {
        AttributeValue::ArrF64(v) => Some(v.clone()),
        AttributeValue::ArrF32(v) => Some(v.iter().map(|x| *x as f64).collect()),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Mapping {
    PolygonVertex,
    ControlPoint,
    Polygon,
    AllSame,
}

struct Layer {
    mapping: Mapping,
    values: Vec<f64>,
    index: Option<Vec<i32>>,
    stride: usize,
}

impl Layer {
    fn read(geo: &NodeHandle, element: &str, values: &str, index: &str, stride: usize) -> Option<Layer> {
        let layer = geo.first_child_by_name(element)?;
        let text = |n: &str| child_attr(&layer, n).and_then(|a| a.get_string()).unwrap_or("");
        let mapping = match text("MappingInformationType") {
            "ByPolygonVertex" => Mapping::PolygonVertex,
            "ByVertice" | "ByVertex" | "ByControlPoint" => Mapping::ControlPoint,
            "ByPolygon" => Mapping::Polygon,
            _ => Mapping::AllSame,
        };
        let indexed = matches!(text("ReferenceInformationType"), "IndexToDirect" | "Index");
        let values = match child_attr(&layer, values)? {
            AttributeValue::ArrI32(v) => v.iter().map(|x| *x as f64).collect(),
            other => floats(other)?,
        };
        let index = indexed
            .then(|| {
                child_attr(&layer, index)
                    .and_then(|a| a.get_arr_i32())
                    .map(<[i32]>::to_vec)
            })
            .flatten();
        Some(Layer {
            mapping,
            values,
            index,
            stride,
        })
    }

    fn get(&self, corner: usize, cp: usize, polygon: usize) -> Option<&[f64]> {
        let i = match self.mapping {
            Mapping::PolygonVertex => corner,
            Mapping::ControlPoint => cp,
            Mapping::Polygon => polygon,
            Mapping::AllSame => 0,
        };
        let i = match &self.index {
            Some(idx) => usize::try_from(*idx.get(i)?).ok()?,
            None => i,
        };
        self.values.get(i * self.stride..(i + 1) * self.stride)
    }
}

pub(super) fn read(geo: &Object) -> Option<GeometryPart> {
    let node = &geo.node;
    let points: Vec<Vec3> = floats(child_attr(node, "Vertices")?)?
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32))
        .collect();
    let polygon_index = child_attr(node, "PolygonVertexIndex")?.get_arr_i32()?;
    let normals = Layer::read(node, "LayerElementNormal", "Normals", "NormalsIndex", 3);
    let uvs = Layer::read(node, "LayerElementUV", "UV", "UVIndex", 2);
    let materials = Layer::read(node, "LayerElementMaterial", "Materials", "", 1);

    let mut part = GeometryPart {
        mesh: Mesh::default(),
        control_point: Vec::new(),
        control_points: points.len(),
        slots: Vec::new(),
    };
    let mut seen: HashMap<(u32, [u32; 3], [u32; 2]), u32> = HashMap::new();
    let mut by_slot: Vec<Vec<u32>> = Vec::new();
    let mut polygon: Vec<u32> = Vec::with_capacity(8);
    let mut polygon_number = 0;

    for (corner, raw) in polygon_index.iter().enumerate() {
        let last = *raw < 0;
        let cp = if last { !*raw } else { *raw } as usize;
        if cp >= points.len() {
            return None;
        }
        let n = normals.as_ref().and_then(|l| l.get(corner, cp, polygon_number));
        let t = uvs.as_ref().and_then(|l| l.get(corner, cp, polygon_number));
        let normal = n.map(|n| Vec3::new(n[0] as f32, n[1] as f32, n[2] as f32));
        // Origine des UV FBX en bas à gauche.
        let uv = t.map(|t| Vec2::new(t[0] as f32, 1.0 - t[1] as f32));
        let key = (
            cp as u32,
            normal.unwrap_or(Vec3::ZERO).to_array().map(f32::to_bits),
            uv.unwrap_or(Vec2::ZERO).to_array().map(f32::to_bits),
        );
        let index = *seen.entry(key).or_insert_with(|| {
            let m = &mut part.mesh;
            m.positions.push(points[cp]);
            m.normals.extend(normal);
            m.uvs.extend(uv);
            part.control_point.push(cp as u32);
            m.positions.len() as u32 - 1
        });
        polygon.push(index);
        if last {
            let slot = materials
                .as_ref()
                .and_then(|l| l.get(corner, cp, polygon_number))
                .map(|v| v[0].max(0.0) as usize)
                .unwrap_or(0);
            if by_slot.len() <= slot {
                by_slot.resize(slot + 1, Vec::new());
            }
            for i in 1..polygon.len().saturating_sub(1) {
                by_slot[slot].extend_from_slice(&[polygon[0], polygon[i], polygon[i + 1]]);
            }
            polygon.clear();
            polygon_number += 1;
        }
    }

    for (slot, indices) in by_slot.into_iter().enumerate() {
        if indices.is_empty() {
            continue;
        }
        let start = part.mesh.indices.len() as u32;
        part.mesh.indices.extend(indices);
        part.slots.push((start..part.mesh.indices.len() as u32, slot));
    }
    let count = part.mesh.positions.len();
    if part.mesh.uvs.len() != count {
        part.mesh.uvs.clear();
    }
    if part.mesh.normals.len() != count {
        part.mesh.normals.clear();
    }
    (!part.mesh.indices.is_empty()).then_some(part)
}
