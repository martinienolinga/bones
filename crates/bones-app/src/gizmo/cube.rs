//! Cube de vue, style Nomad Sculpt : arêtes et coins arrondis, chaque face
//! d'une couleur unie (aucun dégradé ni ombrage), la limite entre deux
//! couleurs passant au milieu de l'arrondi ; libellés noirs en gras
//! italique, inclinés avec leur face.

use std::sync::OnceLock;

use glam::Vec3;

use super::Basis;

/// Rayon des arrondis, relatif à la demi-arête.
const ROUND: f32 = 0.3;
/// Subdivisions par face (les arrondis en ont besoin pour rester lisses).
const STEPS: usize = 10;

struct Face {
    normal: Vec3,
    /// Axe « droite » de la face, pour orienter son libellé.
    u: Vec3,
    label: &'static str,
    color: egui::Color32,
}

/// Couleurs relevées sur le cube de Nomad Sculpt : dessus vert, avant
/// lavande, droite corail (les faces opposées reprennent la même couleur).
const GREEN: egui::Color32 = egui::Color32::from_rgb(0x84, 0xE0, 0x76);
const BLUE: egui::Color32 = egui::Color32::from_rgb(0x82, 0x8A, 0xF5);
const RED: egui::Color32 = egui::Color32::from_rgb(0xF2, 0x68, 0x68);

const fn face(normal: Vec3, u: Vec3, label: &'static str, color: egui::Color32) -> Face {
    Face {
        normal,
        u,
        label,
        color,
    }
}

const FACES: [Face; 6] = [
    face(Vec3::Y, Vec3::X, "Top", GREEN),
    face(Vec3::NEG_Y, Vec3::X, "Bottom", GREEN),
    face(Vec3::Z, Vec3::X, "Front", BLUE),
    face(Vec3::NEG_Z, Vec3::NEG_X, "Back", BLUE),
    face(Vec3::X, Vec3::NEG_Z, "Right", RED),
    face(Vec3::NEG_X, Vec3::Z, "Left", RED),
];

/// Par face : sommets (position, normale) de sa portion du cube arrondi et
/// triangles. Chaque portion s'arrête au milieu des arrondis, d'où des
/// limites de couleur nettes. Calculé une seule fois.
type Patch = (Vec<(Vec3, Vec3)>, Vec<u32>);

fn patches() -> &'static [Patch; 6] {
    static CACHE: OnceLock<[Patch; 6]> = OnceLock::new();
    CACHE.get_or_init(|| {
        let inner = 1.0 - ROUND;
        FACES.each_ref().map(|face| {
            let v = face.normal.cross(face.u);
            let mut vertices = Vec::with_capacity((STEPS + 1) * (STEPS + 1));
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
            let mut indices = Vec::with_capacity(STEPS * STEPS * 6);
            for j in 0..STEPS as u32 {
                for i in 0..STEPS as u32 {
                    let k = j * row + i;
                    indices.extend_from_slice(&[k, k + 1, k + row + 1, k, k + row + 1, k + row]);
                }
            }
            (vertices, indices)
        })
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
    let mut mesh = egui::Mesh::default();
    for (face, (vertices, indices)) in FACES.iter().zip(patches()) {
        if basis.project(face.normal).1 < -0.75 {
            continue; // face entièrement cachée
        }
        let base = mesh.vertices.len() as u32;
        mesh.vertices
            .extend(vertices.iter().map(|&(p, _)| egui::epaint::Vertex {
                pos: center + basis.project(p).0 * half,
                uv: egui::epaint::WHITE_UV,
                color: face.color,
            }));
        // Forme convexe : seuls les triangles vus de face à l'écran (sens de
        // parcours projeté) sont gardés ; ils ne se recouvrent jamais, aucun
        // tri n'est nécessaire et la silhouette n'a pas de trou.
        let screen = &mesh.vertices[base as usize..];
        for tri in indices.as_chunks::<3>().0 {
            let [a, b, c] = tri.map(|i| screen[i as usize].pos);
            if (b - a).x * (c - a).y - (b - a).y * (c - a).x < 0.0 {
                mesh.indices.extend(tri.iter().map(|i| base + i));
            }
        }
    }
    add_fringe(&mut mesh, 1.0 / painter.pixels_per_point());
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
    // Le libellé tient dans la largeur visible de la face (vue de biais, elle rétrécit).
    let room = along.length() * half * 2.0 * 0.78;
    let size = (room / (face.label.chars().count() as f32 * 0.62)).min(15.0);
    if size < 7.0 {
        return;
    }
    let mut job = egui::text::LayoutJob::default();
    job.append(
        face.label,
        0.0,
        egui::TextFormat {
            font_id: bones_ui::theme::semibold(size),
            color: egui::Color32::from_black_alpha((150.0 + 100.0 * facing) as u8),
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

/// Lissage du contour : egui n'adoucit pas les bords des maillages, on
/// ajoute donc autour de la silhouette (enveloppe convexe des sommets, la
/// forme étant convexe) une frange d'un pixel qui passe de la couleur du
/// bord à la transparence, comme le « feathering » d'egui.
fn add_fringe(mesh: &mut egui::Mesh, width: f32) {
    let mut hull = convex_hull(&mesh.vertices);
    // Points quasi confondus : ils donneraient des normales invalides.
    hull.dedup_by(|a, b| a.pos.distance(b.pos) < 0.05);
    let n = hull.len();
    if n < 3 {
        return;
    }
    let base = mesh.vertices.len() as u32;
    for k in 0..n {
        let (prev, cur, next) = (hull[(k + n - 1) % n], hull[k], hull[(k + 1) % n]);
        let outward = |a: egui::Pos2, b: egui::Pos2| {
            let d = (b - a).normalized();
            egui::vec2(d.y, -d.x)
        };
        let normal = (outward(prev.pos, cur.pos) + outward(cur.pos, next.pos)).normalized();
        let normal = if normal.is_finite() {
            normal
        } else {
            egui::Vec2::ZERO
        };
        mesh.vertices.push(cur);
        mesh.vertices.push(egui::epaint::Vertex {
            pos: cur.pos + normal * width,
            uv: cur.uv,
            color: egui::Color32::TRANSPARENT,
        });
    }
    for k in 0..n as u32 {
        let (i, j) = (base + 2 * k, base + 2 * ((k + 1) % n as u32));
        mesh.indices.extend_from_slice(&[i, j, j + 1, i, j + 1, i + 1]);
    }
}

/// Enveloppe convexe (chaîne monotone d'Andrew), dans le sens horaire à
/// l'écran (y vers le bas), ce qui place la normale (dy, −dx) à l'extérieur.
fn convex_hull(points: &[egui::epaint::Vertex]) -> Vec<egui::epaint::Vertex> {
    let mut pts: Vec<egui::epaint::Vertex> = points.to_vec();
    pts.sort_by(|a, b| a.pos.x.total_cmp(&b.pos.x).then(a.pos.y.total_cmp(&b.pos.y)));
    let cross = |o: egui::Pos2, a: egui::Pos2, b: egui::Pos2| (a - o).x * (b - o).y - (a - o).y * (b - o).x;
    let mut hull: Vec<egui::epaint::Vertex> = Vec::with_capacity(64);
    for pass in 0..2 {
        let start = hull.len();
        let iter: Box<dyn Iterator<Item = &egui::epaint::Vertex>> = if pass == 0 {
            Box::new(pts.iter())
        } else {
            Box::new(pts.iter().rev())
        };
        for p in iter {
            while hull.len() >= start + 2
                && cross(hull[hull.len() - 2].pos, hull[hull.len() - 1].pos, p.pos) <= 0.0
            {
                hull.pop();
            }
            hull.push(*p);
        }
        hull.pop();
    }
    hull
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
    fn rounded_patches_stay_in_unit_box_and_meet_mid_edge() {
        for (vertices, indices) in patches() {
            assert_eq!(indices.len(), STEPS * STEPS * 6);
            for (p, n) in vertices {
                assert!(p.abs().max_element() <= 1.0 + 1e-5);
                assert!((n.length() - 1.0).abs() < 1e-4);
            }
        }
        // Le bord de la face +Y rejoint celui de la face +X sur la bissectrice.
        // i = STEPS, j = STEPS / 2 : milieu de l'arête x = +1 de la face +Y.
        let top_edge = patches()[0].0[STEPS / 2 * (STEPS + 1) + STEPS].0;
        assert!((top_edge.x - top_edge.y).abs() < 1e-4, "{top_edge:?}");
    }
}
