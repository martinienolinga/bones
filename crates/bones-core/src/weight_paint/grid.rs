//! Grille régulière en espace écran : retrouve les sommets proches du pinceau
//! sans parcourir tout le maillage. Construite une fois au début du trait à
//! partir des positions écran des sommets déformés.

use glam::Vec2;

pub(crate) struct ScreenGrid {
    origin: Vec2,
    cell: f32,
    cols: i32,
    rows: i32,
    cells: Vec<Vec<u32>>,
}

impl ScreenGrid {
    /// Construit la grille ; `cell` est la taille d'une cellule en pixels
    /// (typiquement le rayon du pinceau). Les sommets hors champ (`None`) sont
    /// ignorés.
    pub fn new(screen: &[Option<Vec2>], cell: f32) -> Self {
        let cell = cell.max(1.0);
        let mut lo = Vec2::splat(f32::INFINITY);
        let mut hi = Vec2::splat(f32::NEG_INFINITY);
        for s in screen.iter().flatten() {
            lo = lo.min(*s);
            hi = hi.max(*s);
        }
        if !lo.is_finite() {
            lo = Vec2::ZERO;
            hi = Vec2::ZERO;
        }
        let cols = (((hi.x - lo.x) / cell).floor() as i32 + 1).max(1);
        let rows = (((hi.y - lo.y) / cell).floor() as i32 + 1).max(1);
        let mut cells = vec![Vec::new(); (cols * rows) as usize];
        for (i, s) in screen.iter().enumerate() {
            if let Some(p) = s {
                let (cx, cy) = (
                    Self::clamp(p.x - lo.x, cell, cols),
                    Self::clamp(p.y - lo.y, cell, rows),
                );
                cells[(cy * cols + cx) as usize].push(i as u32);
            }
        }
        Self {
            origin: lo,
            cell,
            cols,
            rows,
            cells,
        }
    }

    fn clamp(v: f32, cell: f32, count: i32) -> i32 {
        ((v / cell).floor() as i32).clamp(0, count - 1)
    }

    /// Ajoute dans `out` les indices des sommets dont la cellule touche le
    /// disque de centre `center` et de rayon `radius` (surensemble : le
    /// test de distance exact reste à faire par l'appelant).
    pub fn query(&self, center: Vec2, radius: f32, out: &mut Vec<u32>) {
        out.clear();
        let p = center - self.origin;
        let min_x = (((p.x - radius) / self.cell).floor() as i32).clamp(0, self.cols - 1);
        let max_x = (((p.x + radius) / self.cell).floor() as i32).clamp(0, self.cols - 1);
        let min_y = (((p.y - radius) / self.cell).floor() as i32).clamp(0, self.rows - 1);
        let max_y = (((p.y + radius) / self.cell).floor() as i32).clamp(0, self.rows - 1);
        for cy in min_y..=max_y {
            for cx in min_x..=max_x {
                out.extend_from_slice(&self.cells[(cy * self.cols + cx) as usize]);
            }
        }
    }
}
