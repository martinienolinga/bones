//! Projection figée (caméra et zone d'affichage) commune au dessin, à la
//! sélection et aux glissements.

use glam::{Mat4, Vec2, Vec3, Vec4Swizzles};

use crate::{OrbitCamera, Ray, Viewport};

#[derive(Clone, Copy, Debug)]
pub struct Projector {
    camera: OrbitCamera,
    viewport: Viewport,
    view_proj: Mat4,
    eye: Vec3,
    forward: Vec3,
}

impl Projector {
    pub fn new(camera: &OrbitCamera, viewport: &Viewport) -> Self {
        Self {
            camera: *camera,
            viewport: *viewport,
            view_proj: camera.view_proj(viewport),
            eye: camera.eye(),
            forward: camera.forward(),
        }
    }

    pub fn camera(&self) -> &OrbitCamera {
        &self.camera
    }

    /// Point écran (pixels) ; `None` derrière la caméra.
    pub fn try_project(&self, p: Vec3) -> Option<Vec2> {
        let clip = self.view_proj * p.extend(1.0);
        if clip.w <= 1e-6 {
            return None;
        }
        let ndc = clip.xy() / clip.w;
        let vp = &self.viewport;
        Some(Vec2::new(
            vp.x + (ndc.x * 0.5 + 0.5) * vp.width,
            vp.y + (0.5 - ndc.y * 0.5) * vp.height,
        ))
    }

    /// Comme [`Self::try_project`], le manipulateur étant toujours devant la
    /// caméra quand il est construit.
    pub fn project(&self, p: Vec3) -> Vec2 {
        self.try_project(p).unwrap_or(Vec2::ZERO)
    }

    /// Profondeur d'un point le long de l'axe de visée.
    pub fn depth(&self, p: Vec3) -> f32 {
        (p - self.eye).dot(self.forward)
    }

    /// Taille monde d'un pixel à la profondeur de `p`.
    pub fn world_per_px(&self, p: Vec3) -> f32 {
        let c = &self.camera;
        let depth = if c.orthographic { c.distance } else { self.depth(p) };
        2.0 * depth * (c.fov_y * 0.5).tan() / self.viewport.height.max(1.0)
    }

    /// Direction de `p` vers l'observateur.
    pub fn to_eye(&self, p: Vec3) -> Vec3 {
        if self.camera.orthographic {
            -self.forward
        } else {
            (self.eye - p).normalize_or(-self.forward)
        }
    }

    pub fn forward(&self) -> Vec3 {
        self.forward
    }

    pub fn right(&self) -> Vec3 {
        self.camera.right()
    }

    pub fn up(&self) -> Vec3 {
        self.camera.up()
    }

    pub fn ray(&self, screen: Vec2) -> Ray {
        self.camera.ray(screen, &self.viewport)
    }

    /// Point sous `screen` sur le plan passant par `point`, de normale `normal`.
    pub fn on_plane(&self, screen: Vec2, point: Vec3, normal: Vec3) -> Option<Vec3> {
        let ray = self.ray(screen);
        if ray.dir.dot(normal).abs() < 1e-4 {
            return None;
        }
        ray.intersect_plane(point, normal)
    }
}
