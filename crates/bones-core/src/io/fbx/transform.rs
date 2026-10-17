//! Transformations FBX : formule complète des modèles (pivots, pré- et
//! post-rotations, ordre d'Euler) et système d'axes/unités du fichier.

use glam::{DMat3, DMat4, DQuat, DVec3};

use super::document::{Document, Props};

/// Rotation d'Euler FBX (degrés) selon l'ordre `RotationOrder`
/// (0 = XYZ : X appliqué en premier, puis Y, puis Z).
pub(super) fn euler(deg: DVec3, order: i64) -> DQuat {
    let x = DQuat::from_rotation_x(deg.x.to_radians());
    let y = DQuat::from_rotation_y(deg.y.to_radians());
    let z = DQuat::from_rotation_z(deg.z.to_radians());
    match order {
        1 => y * z * x,
        2 => x * z * y,
        3 => z * x * y,
        4 => y * x * z,
        5 => x * y * z,
        _ => z * y * x,
    }
}

/// Parties constantes de la transformation locale d'un modèle ; seules
/// translation, rotation et échelle varient avec l'animation.
pub(super) struct LocalParts {
    pub translation: DVec3,
    pub rotation: DVec3,
    pub scaling: DVec3,
    order: i64,
    before_rotation: DMat4,
    after_rotation: DMat4,
    after_scaling: DMat4,
}

impl LocalParts {
    pub fn new(props: &Props) -> Self {
        let v = |name: &str| props.vec3(name).unwrap_or(DVec3::ZERO);
        let rp = v("RotationPivot");
        let sp = v("ScalingPivot");
        let pre = euler(v("PreRotation"), 0);
        let post = euler(v("PostRotation"), 0);
        Self {
            translation: v("Lcl Translation"),
            rotation: v("Lcl Rotation"),
            scaling: props.vec3("Lcl Scaling").unwrap_or(DVec3::ONE),
            order: props.i64("RotationOrder").unwrap_or(0),
            before_rotation: DMat4::from_translation(v("RotationOffset") + rp) * DMat4::from_quat(pre),
            after_rotation: DMat4::from_quat(post.inverse())
                * DMat4::from_translation(-rp + v("ScalingOffset") + sp),
            after_scaling: DMat4::from_translation(-sp),
        }
    }

    /// L = T · Roff · Rp · Rpre · R · Rpost⁻¹ · Rp⁻¹ · Soff · Sp · S · Sp⁻¹
    pub fn matrix(&self, t: DVec3, r: DVec3, s: DVec3) -> DMat4 {
        DMat4::from_translation(t)
            * self.before_rotation
            * DMat4::from_quat(euler(r, self.order))
            * self.after_rotation
            * DMat4::from_scale(s)
            * self.after_scaling
    }

    pub fn rest(&self) -> DMat4 {
        self.matrix(self.translation, self.rotation, self.scaling)
    }
}

/// Transformation géométrique propre au maillage d'un modèle.
pub(super) fn geometric(props: &Props) -> DMat4 {
    let v = |name: &str| props.vec3(name).unwrap_or(DVec3::ZERO);
    DMat4::from_scale_rotation_translation(
        props.vec3("GeometricScaling").unwrap_or(DVec3::ONE),
        euler(v("GeometricRotation"), 0),
        v("GeometricTranslation"),
    )
}

/// Matrice ramenant le repère du fichier au nôtre (Y en haut, +Z devant,
/// mètres) et cadence d'animation déclarée.
pub(super) fn axis_system(doc: &Document) -> (DMat4, f32) {
    let Some(settings) = &doc.global_settings else {
        return (DMat4::from_scale(DVec3::splat(0.01)), 30.0);
    };
    let p = Props::of(settings);
    let axis = |name: &str, default: i64| p.i64(name).unwrap_or(default).clamp(0, 2) as usize;
    let sign = |name: &str| if p.i64(name).unwrap_or(1) < 0 { -1.0 } else { 1.0 };
    let unit = |i: usize, s: f64| {
        let mut v = DVec3::ZERO;
        v[i] = s;
        v
    };
    let mut x = unit(axis("CoordAxis", 0), sign("CoordAxisSign"));
    let y = unit(axis("UpAxis", 1), sign("UpAxisSign"));
    let z = unit(axis("FrontAxis", 2), sign("FrontAxisSign"));
    // Lignes de la matrice : nos axes exprimés dans le repère du fichier.
    if x.cross(y).dot(z) < 0.0 {
        x = -x;
    }
    let rotation = DMat3::from_cols(x, y, z).transpose();
    let scale = p.f64("UnitScaleFactor").unwrap_or(1.0) / 100.0;
    let fps = match p.i64("TimeMode").unwrap_or(0) {
        1 => 120.0,
        2 => 100.0,
        3 => 60.0,
        4 => 50.0,
        5 => 48.0,
        10 => 25.0,
        11 => 24.0,
        13 => 23.976,
        14 => p.f64("CustomFrameRate").unwrap_or(30.0).max(1.0) as f32,
        15 => 96.0,
        16 => 72.0,
        17 => 59.94,
        8 | 9 => 29.97,
        _ => 30.0,
    };
    (
        DMat4::from_mat3(rotation * DMat3::from_diagonal(DVec3::splat(scale))),
        fps,
    )
}
