//! Objets de scène et primitives : cube de scène (mode Objet, lignes de
//! l'outliner) et les six primitives du mode Objet.
//!
//! Glyphes SVG monochromes (viewBox 32 × 32), dessinés en blanc, dans le
//! style des autres icônes (traits de 2,4 pt, jointures arrondies).

/// Cube isométrique : mode Objet et objets rigides de l'outliner.
pub const OBJECT: &str = r##"<path d="M16 4 25 9.5 16 15 7 9.5Z" stroke="#fff" stroke-width="2.4" stroke-linejoin="round" fill="none"/><path d="M7 9.5V20.5M16 15V26M25 9.5V20.5" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M7 20.5 16 26 25 20.5" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

/// Cube : primitive « Cube » (boîte en 2,5 D).
pub const CUBE: &str = r##"<path d="M8 12H22V26H8Z" stroke="#fff" stroke-width="2.4" stroke-linejoin="round" fill="none"/><path d="M8 12 14 6H28L22 12" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M22 12 28 6V20L22 26" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

/// Sphère : globe avec équateur et méridien.
pub const SPHERE: &str = r##"<circle cx="16" cy="16" r="10.5" stroke="#fff" stroke-width="2.4" fill="none"/><ellipse cx="16" cy="16" rx="10.5" ry="4" stroke="#fff" stroke-width="1.6" fill="none"/><ellipse cx="16" cy="16" rx="4" ry="10.5" stroke="#fff" stroke-width="1.6" fill="none"/>"##;

/// Cylindre : disque du dessus, flancs et base.
pub const CYLINDER: &str = r##"<ellipse cx="16" cy="8.5" rx="8.5" ry="3.2" stroke="#fff" stroke-width="2.4" fill="none"/><path d="M7.5 8.5V23.5M24.5 8.5V23.5" stroke="#fff" stroke-width="2.4" stroke-linecap="round" fill="none"/><path d="M7.5 23.5A8.5 3.2 0 0 0 24.5 23.5" stroke="#fff" stroke-width="2.4" stroke-linecap="round" fill="none"/>"##;

/// Cône : sommet et base elliptique.
pub const CONE: &str = r##"<path d="M16 5 7 23.5M16 5 25 23.5" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><ellipse cx="16" cy="23.5" rx="9" ry="3.2" stroke="#fff" stroke-width="2.4" fill="none"/>"##;

/// Plan : quadrilatère en perspective.
pub const PLANE: &str = r##"<path d="M6 23 13 9 26 9 19 23Z" stroke="#fff" stroke-width="2.4" stroke-linejoin="round" fill="none"/><path d="M9.5 16 22.5 16" stroke="#fff" stroke-width="1.4" stroke-linecap="round" fill="none"/>"##;

/// Tore : anneau (vue du dessus légèrement en perspective).
pub const TORUS: &str = r##"<ellipse cx="16" cy="16" rx="11.5" ry="7.5" stroke="#fff" stroke-width="2.4" fill="none"/><ellipse cx="16" cy="16" rx="4.6" ry="2.8" stroke="#fff" stroke-width="2.4" fill="none"/>"##;
