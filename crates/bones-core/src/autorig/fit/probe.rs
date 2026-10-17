//! Requêtes géométriques sur la surface : segments d'une coupe plane,
//! rayon tiré vers l'arrière depuis l'avant du modèle, composantes connexes.

use glam::{Vec2, Vec3};

use super::surface::Surface;

// Requêtes géométriques utilisées par l'estimation des marqueurs du visage
// (`autorig::face::guess`) et le profil (`autorig::face::profile`).
impl Surface<'_> {
    /// Segments de la coupe par le plan (`point`, `normal`), sans les
    /// regrouper en contours.
    pub fn slice(&self, point: Vec3, normal: Vec3) -> Vec<(Vec3, Vec3)> {
        let side = |v: u32| (self.positions[v as usize] - point).dot(normal);
        self.tris
            .iter()
            .filter_map(|t| {
                let d = t.map(side);
                let above = d.map(|x| x >= 0.0);
                if above[0] == above[1] && above[1] == above[2] {
                    return None;
                }
                let mut ends = (0..3).filter(|k| above[*k] != above[(k + 1) % 3]).map(|k| {
                    let (i, j) = (k, (k + 1) % 3);
                    let s = d[i] / (d[i] - d[j]);
                    self.positions[t[i] as usize].lerp(self.positions[t[j] as usize], s)
                });
                Some((ends.next()?, ends.next()?))
            })
            .collect()
    }

    /// Premier point de la surface rencontré en venant de l'avant (+Z) le
    /// long de la droite verticale à l'écran passant par (`x`, `y`) :
    /// le point visible de face.
    pub fn front_hit(&self, x: f32, y: f32) -> Option<Vec3> {
        let q = Vec2::new(x, y);
        self.tris
            .iter()
            .filter_map(|t| {
                let [a, b, c] = t.map(|v| self.positions[v as usize]);
                let (a2, b2, c2) = (a.truncate(), b.truncate(), c.truncate());
                let area = (b2 - a2).perp_dot(c2 - a2);
                if area.abs() < 1e-12 {
                    return None;
                }
                let u = (c2 - b2).perp_dot(q - b2) / area;
                let v = (a2 - c2).perp_dot(q - c2) / area;
                let w = 1.0 - u - v;
                (u >= 0.0 && v >= 0.0 && w >= 0.0).then(|| a * u + b * v + c * w)
            })
            .max_by(|p, q| p.z.total_cmp(&q.z))
    }

    /// Composantes connexes (sommets utilisés par des triangles).
    pub fn components(&self) -> Vec<Vec<u32>> {
        let mut seen = vec![false; self.positions.len()];
        let mut out = Vec::new();
        for start in self.vertices() {
            if seen[start as usize] {
                continue;
            }
            seen[start as usize] = true;
            let mut stack = vec![start];
            let mut component = Vec::new();
            while let Some(v) = stack.pop() {
                component.push(v);
                for &u in self.neighbors(v) {
                    if !seen[u as usize] {
                        seen[u as usize] = true;
                        stack.push(u);
                    }
                }
            }
            out.push(component);
        }
        out
    }
}
