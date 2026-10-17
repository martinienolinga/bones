//! Courbes utilitaires des générateurs : adoucissements, trajectoire à jerk
//! minimal, bosse, bruit périodique (bouclable) et courbes à clés.

use std::f32::consts::{PI, TAU};

/// Adoucissement cubique sur [0, 1].
pub(crate) fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Trajectoire à jerk minimal (vitesse et accélération nulles aux bouts),
/// celle des mouvements humains volontaires (Flash & Hogan, 1985).
pub(crate) fn minjerk(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t * (10.0 + t * (-15.0 + 6.0 * t))
}

/// Bosse sur [0, 1] (nulle aux bouts) culminant à `peak` (0 < peak < 1).
pub(crate) fn bump(u: f32, peak: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    let k = 0.5f32.ln() / peak.clamp(0.05, 0.95).ln();
    (PI * u.powf(k)).sin()
}

/// Sinus d'une phase exprimée en tours.
pub(crate) fn wave(turns: f32) -> f32 {
    (TAU * turns).sin()
}

/// Cosinus d'une phase exprimée en tours.
pub(crate) fn cwave(turns: f32) -> f32 {
    (TAU * turns).cos()
}

/// Bruit doux et périodique de période 1 (somme d'harmoniques entières
/// aux phases pseudo-aléatoires), dans [−1, 1] environ.
pub(crate) fn noise(u: f32, seed: u32, harmonics: &[u32]) -> f32 {
    let mut sum = 0.0;
    let mut weight = 0.0;
    for (i, h) in harmonics.iter().enumerate() {
        let phase = hash(seed.wrapping_mul(31).wrapping_add(i as u32));
        let a = 1.0 / (1.0 + i as f32);
        sum += a * wave(*h as f32 * u + phase);
        weight += a;
    }
    sum / weight.max(1e-6)
}

/// Nombre pseudo-aléatoire dans [0, 1) dérivé d'un entier.
pub(crate) fn hash(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x & 0xFF_FFFF) as f32 / 16_777_216.0
}

/// Courbe à clés (temps, valeur) : Hermite cubique, tangentes de
/// Catmull-Rom annulées aux extremums et aux paliers (pas de dépassement).
pub(crate) struct Curve {
    keys: Vec<(f32, f32)>,
}

impl Curve {
    pub fn new(keys: &[(f32, f32)]) -> Self {
        let mut keys = keys.to_vec();
        keys.sort_by(|a, b| a.0.total_cmp(&b.0));
        Self { keys }
    }

    fn tangent(&self, i: usize) -> f32 {
        let k = &self.keys;
        if i == 0 || i + 1 >= k.len() {
            return 0.0;
        }
        let (prev, cur, next) = (k[i - 1], k[i], k[i + 1]);
        if (cur.1 - prev.1) * (next.1 - cur.1) <= 0.0 {
            return 0.0;
        }
        (next.1 - prev.1) / (next.0 - prev.0).max(1e-6)
    }

    pub fn at(&self, t: f32) -> f32 {
        let k = &self.keys;
        match k.len() {
            0 => return 0.0,
            1 => return k[0].1,
            _ => {}
        }
        if t <= k[0].0 {
            return k[0].1;
        }
        if t >= k[k.len() - 1].0 {
            return k[k.len() - 1].1;
        }
        let i = k.partition_point(|key| key.0 <= t).max(1) - 1;
        let (t0, p0) = k[i];
        let (t1, p1) = k[i + 1];
        let dt = (t1 - t0).max(1e-6);
        let s = (t - t0) / dt;
        let (m0, m1) = (self.tangent(i) * dt, self.tangent(i + 1) * dt);
        let (s2, s3) = (s * s, s * s * s);
        p0 * (2.0 * s3 - 3.0 * s2 + 1.0) + m0 * (s3 - 2.0 * s2 + s) + p1 * (3.0 * s2 - 2.0 * s3) + m1 * (s3 - s2)
    }
}

/// Polynôme d'Hermite : position à `t` ∈ [0, dt] partant de `p0` à la
/// vitesse `v0` pour arriver en `p1` à la vitesse `v1`.
pub(crate) fn hermite(p0: f32, v0: f32, p1: f32, v1: f32, t: f32, dt: f32) -> f32 {
    let s = (t / dt.max(1e-6)).clamp(0.0, 1.0);
    let (s2, s3) = (s * s, s * s * s);
    p0 * (2.0 * s3 - 3.0 * s2 + 1.0)
        + v0 * dt * (s3 - 2.0 * s2 + s)
        + p1 * (3.0 * s2 - 2.0 * s3)
        + v1 * dt * (s3 - s2)
}

/// Enveloppe supérieure d'un signal périodique échantillonné, à
/// accélération bornée (dilatation par une parabole) : la courbe obtenue
/// reste au-dessus de `need`, sans changement de vitesse brusque.
pub(crate) fn envelope(need: &[f32], dt: f32, accel: f32) -> Vec<f32> {
    let n = need.len();
    (0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    let k = i.abs_diff(j).min(n - i.abs_diff(j)) as f32 * dt;
                    need[j] - 0.5 * accel * k * k
                })
                .fold(f32::MIN, f32::max)
        })
        .collect()
}

/// Valeur d'un signal périodique échantillonné (période 1) à `u`,
/// interpolée par Catmull-Rom.
pub(crate) fn periodic(samples: &[f32], u: f32) -> f32 {
    let n = samples.len();
    if n == 0 {
        return 0.0;
    }
    let x = u.rem_euclid(1.0) * n as f32;
    let i = x.floor() as usize;
    let t = x - i as f32;
    let p = |k: usize| samples[(i + n + k - 1) % n];
    let (p0, p1, p2, p3) = (p(0), p(1), p(2), p(3));
    0.5 * (2.0 * p1
        + (p2 - p0) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t * t
        + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t * t * t)
}
