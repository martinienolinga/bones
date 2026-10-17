//! Transformations affines décomposées (translation, rotation, échelle).

use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Transformation affine décomposée (translation, rotation, échelle).
///
/// La composition et l'inversion sont exactes tant que l'échelle est
/// uniforme, ce qui est toujours le cas pour les os édités dans l'application.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            ..Self::IDENTITY
        }
    }

    pub fn from_rotation_translation(rotation: Quat, translation: Vec3) -> Self {
        Self {
            translation,
            rotation,
            scale: Vec3::ONE,
        }
    }

    pub fn from_matrix(m: Mat4) -> Self {
        let (scale, rotation, translation) = m.to_scale_rotation_translation();
        Self {
            translation,
            rotation: rotation.normalize(),
            scale,
        }
    }

    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// `self * child` : applique `child` puis `self`.
    pub fn mul(&self, child: &Transform) -> Transform {
        Transform {
            translation: self.transform_point(child.translation),
            rotation: (self.rotation * child.rotation).normalize(),
            scale: self.scale * child.scale,
        }
    }

    pub fn inverse(&self) -> Transform {
        let rotation = self.rotation.inverse();
        let scale = self.scale.recip();
        Transform {
            translation: rotation * (-self.translation) * scale,
            rotation,
            scale,
        }
    }

    pub fn transform_point(&self, p: Vec3) -> Vec3 {
        self.translation + self.rotation * (self.scale * p)
    }

    pub fn transform_vector(&self, v: Vec3) -> Vec3 {
        self.rotation * (self.scale * v)
    }

    /// Interpolation : linéaire pour la translation et l'échelle, sphérique
    /// pour la rotation.
    pub fn lerp(&self, other: &Transform, t: f32) -> Transform {
        Transform {
            translation: self.translation.lerp(other.translation, t),
            rotation: self.rotation.slerp(other.rotation, t).normalize(),
            scale: self.scale.lerp(other.scale, t),
        }
    }

    pub fn abs_diff_eq(&self, other: &Transform, eps: f32) -> bool {
        self.translation.abs_diff_eq(other.translation, eps)
            && (self.rotation.abs_diff_eq(other.rotation, eps)
                || self.rotation.abs_diff_eq(-other.rotation, eps))
            && self.scale.abs_diff_eq(other.scale, eps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    #[test]
    fn mul_matches_matrices() {
        let a = Transform {
            translation: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::from_rotation_z(0.7),
            scale: Vec3::splat(2.0),
        };
        let b = Transform {
            translation: Vec3::new(-1.0, 0.5, 0.0),
            rotation: Quat::from_rotation_x(FRAC_PI_2),
            scale: Vec3::splat(0.5),
        };
        let composed = a.mul(&b).to_matrix();
        let expected = a.to_matrix() * b.to_matrix();
        assert!(composed.abs_diff_eq(expected, 1e-5));
    }

    #[test]
    fn inverse_roundtrip() {
        let a = Transform {
            translation: Vec3::new(1.0, -2.0, 0.25),
            rotation: Quat::from_euler(glam::EulerRot::XYZ, 0.3, 1.1, -0.4),
            scale: Vec3::splat(1.5),
        };
        let id = a.mul(&a.inverse());
        assert!(id.abs_diff_eq(&Transform::IDENTITY, 1e-5));
        let p = Vec3::new(0.3, 0.2, -5.0);
        assert!(
            a.inverse()
                .transform_point(a.transform_point(p))
                .abs_diff_eq(p, 1e-4)
        );
    }
}
