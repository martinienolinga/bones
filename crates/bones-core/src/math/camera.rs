//! Caméra orbitale, zone d'affichage et lancer de rayons.

use glam::{Mat4, Vec2, Vec3, Vec4Swizzles};
use serde::{Deserialize, Serialize};

/// Zone d'affichage 3D, en pixels physiques (origine en haut à gauche).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Viewport {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width,
            height,
        }
    }

    pub fn aspect(&self) -> f32 {
        self.width / self.height.max(1.0)
    }

    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.x && p.y >= self.y && p.x < self.x + self.width && p.y < self.y + self.height
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl Ray {
    pub fn intersect_plane(&self, point: Vec3, normal: Vec3) -> Option<Vec3> {
        let denom = self.dir.dot(normal);
        if denom.abs() < 1e-6 {
            return None;
        }
        let t = (point - self.origin).dot(normal) / denom;
        (t >= 0.0).then(|| self.origin + self.dir * t)
    }
}

/// Caméra orbitale, pilotée au doigt comme dans Nomad Sculpt.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrbitCamera {
    pub target: Vec3,
    pub distance: f32,
    /// Rotation autour de Y (radians). 0 : la caméra regarde vers -Z.
    pub yaw: f32,
    /// Élévation (radians), bornée pour ne jamais passer par les pôles.
    pub pitch: f32,
    pub fov_y: f32,
    /// Rotation verrouillée : glisser ne fait plus tourner la vue.
    #[serde(default)]
    pub locked: bool,
    /// Projection orthographique (vues de face ou de profil exactes).
    #[serde(default)]
    pub orthographic: bool,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            target: Vec3::new(0.0, 1.0, 0.0),
            distance: 4.0,
            yaw: 0.35,
            pitch: 0.2,
            fov_y: 45f32.to_radians(),
            locked: false,
            orthographic: false,
        }
    }
}

/// Élévation maximale (vue de dessus / de dessous), juste avant le pôle.
pub const PITCH_LIMIT: f32 = 1.55;

impl OrbitCamera {
    pub fn eye(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        self.target + Vec3::new(cp * sy, sp, cp * cy) * self.distance
    }

    /// Direction de visée (de l'œil vers la cible).
    pub fn forward(&self) -> Vec3 {
        (self.target - self.eye()).normalize()
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    pub fn up(&self) -> Vec3 {
        self.right().cross(self.forward())
    }

    pub fn view(&self) -> Mat4 {
        glam::camera::rh::view::look_at_mat4(self.eye(), self.target, Vec3::Y)
    }

    /// Projection (perspective ou orthographique), NDC Z dans [0, 1] et Y
    /// vers le haut (convention wgpu/Metal). En orthographique, la distance
    /// règle seulement l'échelle : la même que la perspective à la cible.
    pub fn projection(&self, aspect: f32) -> Mat4 {
        let far = self.distance * 100.0 + 100.0;
        if self.orthographic {
            let h = self.distance * (self.fov_y * 0.5).tan();
            let w = h * aspect;
            return glam::camera::rh::proj::directx::orthographic(-w, w, -h, h, -far, far);
        }
        let near = (self.distance * 0.01).max(1e-3);
        glam::camera::rh::proj::directx::perspective(self.fov_y, aspect, near, far)
    }

    pub fn view_proj(&self, viewport: &Viewport) -> Mat4 {
        self.projection(viewport.aspect()) * self.view()
    }

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        if self.locked {
            return;
        }
        self.yaw -= dx * 0.008;
        self.pitch = (self.pitch + dy * 0.008).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    /// Déplacement latéral : le contenu suit le doigt.
    pub fn pan(&mut self, dx: f32, dy: f32, viewport: &Viewport) {
        let world_per_px = 2.0 * self.distance * (self.fov_y * 0.5).tan() / viewport.height.max(1.0);
        self.target += (-self.right() * dx + self.up() * dy) * world_per_px;
    }

    /// `factor` > 1 rapproche la caméra.
    pub fn zoom(&mut self, factor: f32) {
        if factor.is_finite() && factor > 0.0 {
            self.distance = (self.distance / factor).clamp(0.05, 5000.0);
        }
    }

    /// Cadre une boîte englobante.
    pub fn frame(&mut self, min: Vec3, max: Vec3) {
        let radius = ((max - min).length() * 0.5).max(0.01);
        self.target = (min + max) * 0.5;
        self.distance = radius / (self.fov_y * 0.5).sin() * 1.1;
    }

    /// Projection d'un point monde en pixels écran ; `None` s'il est derrière la caméra.
    pub fn project(&self, p: Vec3, viewport: &Viewport) -> Option<Vec2> {
        let clip = self.view_proj(viewport) * p.extend(1.0);
        if clip.w <= 1e-6 {
            return None;
        }
        let ndc = clip.xy() / clip.w;
        Some(Vec2::new(
            viewport.x + (ndc.x * 0.5 + 0.5) * viewport.width,
            viewport.y + (0.5 - ndc.y * 0.5) * viewport.height,
        ))
    }

    /// Rayon partant de l'œil et passant par un pixel écran.
    pub fn ray(&self, screen: Vec2, viewport: &Viewport) -> Ray {
        let ndc = Vec2::new(
            (screen.x - viewport.x) / viewport.width * 2.0 - 1.0,
            1.0 - (screen.y - viewport.y) / viewport.height * 2.0,
        );
        let inv = self.view_proj(viewport).inverse();
        let near = inv.project_point3(ndc.extend(0.0));
        let far = inv.project_point3(ndc.extend(1.0));
        Ray {
            origin: near,
            dir: (far - near).normalize(),
        }
    }

    /// Point sous le doigt, sur le plan parallèle à l'écran passant par `anchor`.
    pub fn point_on_view_plane(&self, screen: Vec2, viewport: &Viewport, anchor: Vec3) -> Vec3 {
        self.ray(screen, viewport)
            .intersect_plane(anchor, self.forward())
            .unwrap_or(anchor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_and_ray_agree() {
        let cam = OrbitCamera::default();
        let vp = Viewport {
            x: 10.0,
            y: 20.0,
            width: 800.0,
            height: 600.0,
        };
        let p = Vec3::new(0.3, 1.2, -0.4);
        let s = cam.project(p, &vp).unwrap();
        let back = cam.point_on_view_plane(s, &vp, p);
        assert!(back.abs_diff_eq(p, 1e-3));
        let target = cam.project(cam.target, &vp).unwrap();
        assert!(target.abs_diff_eq(Vec2::new(410.0, 320.0), 1e-2));
    }

    #[test]
    fn pan_follows_finger() {
        let mut cam = OrbitCamera::default();
        let vp = Viewport::new(800.0, 600.0);
        let p = Vec3::new(0.2, 0.9, 0.1);
        let before = cam.project(p, &vp).unwrap();
        cam.pan(50.0, -30.0, &vp);
        let after = cam.project(p, &vp).unwrap();
        // Projection perspective : le point suit le doigt à peu de chose près.
        assert!((after - before).abs_diff_eq(Vec2::new(50.0, -30.0), 3.0));
    }

    #[test]
    fn orthographic_project_and_ray_agree() {
        let cam = OrbitCamera {
            orthographic: true,
            ..Default::default()
        };
        let vp = Viewport::new(800.0, 600.0);
        let p = Vec3::new(0.3, 1.2, -0.4);
        let s = cam.project(p, &vp).unwrap();
        assert!(cam.point_on_view_plane(s, &vp, p).abs_diff_eq(p, 1e-3));
        // Rayons parallèles : même direction partout à l'écran.
        let a = cam.ray(Vec2::new(10.0, 10.0), &vp).dir;
        let b = cam.ray(Vec2::new(700.0, 500.0), &vp).dir;
        assert!(a.abs_diff_eq(b, 1e-4));
    }

    #[test]
    fn locked_camera_does_not_orbit() {
        let mut cam = OrbitCamera {
            locked: true,
            ..Default::default()
        };
        let before = cam;
        cam.orbit(100.0, 50.0);
        assert_eq!(cam, before);
    }
}
