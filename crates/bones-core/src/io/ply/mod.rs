//! Stanford PLY (ASCII, binaire petit ou grand boutisme) : positions,
//! normales, coordonnées de texture et faces polygonales (triangulées).
//! Export binaire du maillage posé.

mod header;

use glam::{Vec2, Vec3};

use header::{Property, Values, err, parse_header};

use super::IoError;
use super::rigging::posed_mesh;
use crate::{Mesh, Rig};

/// Indices des propriétés de sommet utiles.
#[derive(Default)]
struct VertexLayout {
    pos: [Option<usize>; 3],
    normal: [Option<usize>; 3],
    uv: [Option<usize>; 2],
}

impl VertexLayout {
    fn new(props: &[Property]) -> Self {
        let mut l = VertexLayout::default();
        for (i, p) in props.iter().enumerate() {
            let Property::Scalar(name, _) = p else { continue };
            match name.as_str() {
                "x" => l.pos[0] = Some(i),
                "y" => l.pos[1] = Some(i),
                "z" => l.pos[2] = Some(i),
                "nx" => l.normal[0] = Some(i),
                "ny" => l.normal[1] = Some(i),
                "nz" => l.normal[2] = Some(i),
                "u" | "s" | "texture_u" | "texture_s" => l.uv[0] = Some(i),
                "v" | "t" | "texture_v" | "texture_t" => l.uv[1] = Some(i),
                _ => {}
            }
        }
        l
    }
}

pub(super) fn import(bytes: &[u8]) -> Result<Mesh, IoError> {
    let header = parse_header(bytes)?;
    let mut values = Values::new(&header, bytes);
    let mut mesh = Mesh::default();
    let mut row: Vec<f64> = Vec::new();

    for element in &header.elements {
        let layout = VertexLayout::new(&element.properties);
        let is_vertex = element.name == "vertex";
        let is_face = element.name == "face";
        for _ in 0..element.count {
            row.clear();
            let mut polygon: Vec<u32> = Vec::new();
            for p in &element.properties {
                match p {
                    Property::Scalar(_, ty) => row.push(values.read(*ty)?),
                    Property::List(name, count_ty, item_ty) => {
                        let n = values.read(*count_ty)? as usize;
                        let wanted = is_face && (name == "vertex_indices" || name == "vertex_index");
                        for _ in 0..n {
                            let v = values.read(*item_ty)?;
                            if wanted {
                                polygon.push(v as u32);
                            }
                        }
                        row.push(0.0);
                    }
                }
            }
            if is_vertex {
                let get = |i: Option<usize>| i.map(|i| row[i] as f32);
                let [x, y, z] = layout.pos.map(get);
                mesh.positions
                    .push(Vec3::new(x.unwrap_or(0.0), y.unwrap_or(0.0), z.unwrap_or(0.0)));
                if let [Some(nx), Some(ny), Some(nz)] = layout.normal.map(get) {
                    mesh.normals.push(Vec3::new(nx, ny, nz));
                }
                if let [Some(u), Some(v)] = layout.uv.map(get) {
                    mesh.uvs.push(Vec2::new(u, 1.0 - v));
                }
            } else if is_face && polygon.len() >= 3 {
                for i in 1..polygon.len() - 1 {
                    mesh.indices
                        .extend_from_slice(&[polygon[0], polygon[i], polygon[i + 1]]);
                }
            }
        }
    }

    let n = mesh.positions.len() as u32;
    if mesh.indices.is_empty() {
        return Err(err("no face"));
    }
    if mesh.indices.iter().any(|i| *i >= n) {
        return Err(err("vertex index out of range"));
    }
    if mesh.normals.len() != mesh.positions.len() {
        mesh.compute_normals();
    }
    if mesh.uvs.len() != mesh.positions.len() {
        mesh.uvs.clear();
    }
    mesh.shrink_to_fit();
    Ok(mesh)
}

/// PLY binaire (petit boutisme) du maillage dans la pose courante.
pub(super) fn export(rig: &Rig) -> Vec<u8> {
    let mesh = posed_mesh(rig);
    let uvs = mesh.has_uvs();
    let mut header = format!(
        "ply\nformat binary_little_endian 1.0\ncomment PaintSharp Bones\nelement vertex {}\n\
         property float x\nproperty float y\nproperty float z\n\
         property float nx\nproperty float ny\nproperty float nz\n",
        mesh.vertex_count()
    );
    if uvs {
        header.push_str("property float s\nproperty float t\n");
    }
    header.push_str(&format!(
        "element face {}\nproperty list uchar int vertex_indices\nend_header\n",
        mesh.triangle_count()
    ));
    let stride = if uvs { 32 } else { 24 };
    let mut out =
        Vec::with_capacity(header.len() + mesh.vertex_count() * stride + mesh.triangle_count() * 13);
    out.extend_from_slice(header.as_bytes());
    for (i, (p, n)) in mesh.positions.iter().zip(&mesh.normals).enumerate() {
        for f in p.to_array().into_iter().chain(n.to_array()) {
            out.extend_from_slice(&f.to_le_bytes());
        }
        if uvs {
            let t = mesh.uvs[i];
            out.extend_from_slice(&t.x.to_le_bytes());
            out.extend_from_slice(&(1.0 - t.y).to_le_bytes());
        }
    }
    for tri in mesh.indices.as_chunks::<3>().0 {
        out.push(3);
        for i in tri {
            out.extend_from_slice(&(*i as i32).to_le_bytes());
        }
    }
    out
}
