//! Cube de vue, style Nomad Sculpt : cube aux arêtes et coins arrondis,
//! ombré, faces aux couleurs de leur axe (fondues le long des arrondis),
//! libellés en italique inclinés avec leur face.

use std::sync::OnceLock;

use glam::Vec3;

use super::Basis;

/// Rayon des arrondis, relatif à la demi-arête.
const ROUND: f32 = 0.32;
/// Subdivisions par face (les arrondis en ont besoin pour rester lisses).
const STEPS: usize = 10;

struct Face {
    normal: Vec3,
    /// Axe « droite » de la face, pour orienter son libellé.
    u: Vec3,
    label: &'static str,
}

const FACES: [Face; 6] = [
    Face {
        normal: Vec3::Y,
        u: Vec3::X,
        label: "Dessus",
    },
    Face {
        normal: Vec3::NEG_Y,
        u: Vec3::X,
        label: "Dessous",
    },
    Face {
        normal: Vec3::Z,
        u: Vec3::X,
        label: "Avant",
    },
    Face {
        normal: Vec3::NEG_Z,
        u: Vec3::NEG_X,
        label: "Arrière",
    },
    Face {
        normal: Vec3::X,
        u: Vec3::NEG_Z,
        label: "Droite",
    },
    Face {
        normal: Vec3::NEG_X,
        u: Vec3::Z,
        label: "Gauche",
    },
];

/// Couleurs pastel façon Nomad : X corail, Y vert tendre, Z lavande ; les
/// faces négatives sont un peu plus sombres.
fn axis_color(normal: Vec3) -> Vec3 {
    let (x, y, z) = (
        Vec3::new(0.95, 0.42, 0.42),
        Vec3::new(0.49, 0.84, 0.45),
        Vec3::new(0.49, 0.53, 0.96),
    );
    let pick = |v: f32, c: Vec3| if v >= 0.0 { c } else { c * 0.78 };
    // Poids à la puissance 4 : couleur franche sur la face, fondu limité à l'arrondi.
    let w = normal * normal * normal * normal;
    let w = w / (w.x + w.y + w.z);
    pick(normal.x, x) * w.x + pick(normal.y, y) * w.y + pick(normal.z, z) * w.z
}

type Geometry = (Vec<(Vec3, Vec3)>, Vec<u32>);

/// Sommets (position, normale) et triangles du cube arrondi, calculés une fois.
fn geometry() -> &'static Geometry {
    static CACHE: OnceLock<Geometry> = OnceLock::new();
    CACHE.get_or_init(|| {
        let inner = 1.0 - ROUND;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for face in &FACES {
            let v = face.normal.cross(face.u);
            let base = vertices.len() as u32;
            for j in 0..=STEPS {
                for i in 0..=STEPS {
                    let a = i as f32 / STEPS as f32 * 2.0 - 1.0;
                    let b = j as f32 / STEPS as f32 * 2.0 - 1.0;
                    let p = face.normal + face.u * a + v * b;
                    // Projection sur la boîte arrondie : noyau + rayon.
                    let core = p.clamp(Vec3::splat(-inner), Vec3::splat(inner));
                    let n = (p - core).normalize_or(face.normal);
                    vertices.push((core + n * ROUND, n));
                }
            }
            let row = STEPS as u32 + 1;
            for j in 0..STEPS as u32 {
                for i in 0..STEPS as u32 {
                    let k = base + j * row + i;
                    indices.extend_from_slice(&[k, k + 1, k + row + 1, k, k + row + 1, k + row]);
                }
            }
        }
        (vertices, indices)
    })
}

/// Dessine le cube ; retourne la normale de la face sous `hit`.
pub fn paint(
    painter: &egui::Painter,
    center: egui::Pos2,
    half: f32,
    basis: &Basis,
    hit: Option<egui::Pos2>,
) -> Option<Vec3> {
    let (vertices, indices) = geometry();
    // Lumière douce venant d'en haut à gauche, côté observateur.
    let light =
        (basis.world_toward_viewer() + basis.world_up() * 0.6 - basis.world_right() * 0.4).normalize();
    let mut mesh = egui::Mesh::default();
    mesh.vertices.reserve(vertices.len());
    for &(p, n) in vertices {
        let shade = 0.58 + 0.42 * n.dot(light).max(0.0);
        let c = (axis_color(n) * shade).clamp(Vec3::ZERO, Vec3::ONE) * 255.0;
        mesh.vertices.push(egui::epaint::Vertex {
            pos: center + basis.project(p).0 * half,
            uv: egui::epaint::WHITE_UV,
            color: egui::Color32::from_rgb(c.x as u8, c.y as u8, c.z as u8),
        });
    }
    // Forme convexe : les triangles tournés vers l'observateur ne se
    // recouvrent jamais, aucun tri n'est nécessaire.
    for tri in indices.as_chunks::<3>().0 {
        let n: Vec3 = tri.iter().map(|&i| vertices[i as usize].1).sum();
        if basis.project(n).1 > 0.0 {
            mesh.indices.extend_from_slice(tri);
        }
    }
    painter.add(egui::Shape::mesh(mesh));

    let mut picked = None;
    for face in &FACES {
        let facing = basis.project(face.normal).1;
        if facing <= 0.05 {
            continue;
        }
        if facing > 0.3 {
            label(painter, center, half, basis, face, facing);
        }
        let v = face.normal.cross(face.u);
        let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .map(|(a, b)| center + basis.project(face.normal + face.u * a + v * b).0 * half);
        if hit.is_some_and(|h| inside(&corners, h)) {
            picked = Some(face.normal);
        }
    }
    picked
}

fn label(painter: &egui::Painter, center: egui::Pos2, half: f32, basis: &Basis, face: &Face, facing: f32) {
    let pos = center + basis.project(face.normal).0 * half;
    let along = basis.project(face.u).0;
    let mut job = egui::text::LayoutJob::default();
    job.append(
        face.label,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::proportional(10.0 + 4.0 * facing),
            color: egui::Color32::from_black_alpha((140.0 + 100.0 * facing) as u8),
            italics: true,
            ..Default::default()
        },
    );
    let galley = painter.layout_job(job);
    // Le pivot de rotation est le centre du texte, placé au centre de la face.
    let top_left = pos - galley.size() * 0.5;
    painter.add(
        egui::epaint::TextShape::new(top_left, galley, egui::Color32::BLACK)
            .with_angle_and_anchor(along.y.atan2(along.x), egui::Align2::CENTER_CENTER),
    );
}

/// Point dans un quadrilatère convexe (sens quelconque).
fn inside(quad: &[egui::Pos2; 4], p: egui::Pos2) -> bool {
    let mut sign = 0.0f32;
    for i in 0..4 {
        let (a, b) = (quad[i], quad[(i + 1) % 4]);
        let cross = (b - a).x * (p - a).y - (b - a).y * (p - a).x;
        if cross.abs() < 1e-6 {
            continue;
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if cross.signum() != sign {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_in_quad() {
        let q = [
            egui::pos2(0.0, 0.0),
            egui::pos2(10.0, 0.0),
            egui::pos2(10.0, 10.0),
            egui::pos2(0.0, 10.0),
        ];
        assert!(inside(&q, egui::pos2(5.0, 5.0)));
        assert!(!inside(&q, egui::pos2(15.0, 5.0)));
    }

    #[test]
    fn rounded_cube_stays_in_unit_box_with_unit_normals() {
        let (vertices, indices) = geometry();
        assert_eq!(indices.len(), 6 * STEPS * STEPS * 6);
        for (p, n) in vertices {
            assert!(p.abs().max_element() <= 1.0 + 1e-5);
            assert!((n.length() - 1.0).abs() < 1e-4);
        }
    }
}
