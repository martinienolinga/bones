//! Icônes propres au rigging et à l'animation.

use super::pen::Pen;

/// Os classique : tige et deux paires de rotules.
pub(super) fn bone(p: &Pen) {
    p.line(&[(7.5, 16.5), (16.5, 7.5)]);
    for c in [(5.6, 15.6), (8.4, 18.4), (15.6, 5.6), (18.4, 8.4)] {
        p.circle(c, 2.3);
    }
}

/// Os octaédrique à la Blender, de la tête `h` vers la queue `t`.
fn octa(p: &Pen, h: (f32, f32), t: (f32, f32)) {
    let (dx, dy) = (t.0 - h.0, t.1 - h.1);
    let w = (h.0 + dx * 0.22, h.1 + dy * 0.22);
    let (nx, ny) = (-dy * 0.16, dx * 0.16);
    let (a, b) = ((w.0 + nx, w.1 + ny), (w.0 - nx, w.1 - ny));
    p.closed(&[h, a, t, b]);
    p.line(&[a, b]);
}

pub(super) fn edit(p: &Pen) {
    octa(p, (5.0, 19.0), (19.0, 5.0));
}

pub(super) fn bone_add(p: &Pen) {
    octa(p, (4.0, 20.0), (14.0, 10.0));
    p.line(&[(18.0, 3.0), (18.0, 11.0)]);
    p.line(&[(14.0, 7.0), (22.0, 7.0)]);
}

/// Silhouette en mouvement (mode pose).
pub(super) fn pose(p: &Pen) {
    p.circle((12.0, 4.5), 2.3);
    p.line(&[(12.0, 7.5), (11.5, 14.0)]);
    p.line(&[(5.5, 8.0), (12.0, 9.5), (18.0, 5.5)]);
    p.line(&[(7.0, 21.0), (11.5, 14.0), (14.5, 17.0), (17.5, 21.0)]);
}

pub(super) fn move_tool(p: &Pen) {
    p.line(&[(12.0, 4.5), (12.0, 19.5)]);
    p.line(&[(4.5, 12.0), (19.5, 12.0)]);
    for (tip, dir) in [
        ((12.0, 3.0), -90.0),
        ((12.0, 21.0), 90.0),
        ((3.0, 12.0), 180.0),
        ((21.0, 12.0), 0.0),
    ] {
        p.arrow_head(tip, dir, 3.5);
    }
}

/// Flèche circulaire horaire autour d'un pivot.
pub(super) fn rotate(p: &Pen) {
    p.arc((12.0, 12.0), 7.5, -45.0, 215.0);
    p.arrow_head((8.6, 5.0), -45.0, 3.8);
    p.dot((12.0, 12.0), 1.8);
}

/// Flèche circulaire antihoraire et aiguilles (retour à la pose de repos).
pub(super) fn reset(p: &Pen) {
    p.arc((12.0, 12.5), 7.5, -35.0, 225.0);
    p.arrow_head((16.0, 5.6), -135.0, 3.8);
    p.line(&[(12.0, 8.5), (12.0, 12.5), (14.5, 14.5)]);
}

pub(super) fn pin(p: &Pen) {
    p.arc((12.0, 9.5), 5.5, 150.0, 390.0);
    p.line(&[(7.2, 12.2), (12.0, 21.0), (16.8, 12.2)]);
    p.dot((12.0, 9.5), 1.8);
}

pub(super) fn select(p: &Pen) {
    p.closed(&[
        (6.0, 3.5),
        (6.0, 19.0),
        (10.0, 15.0),
        (13.0, 21.0),
        (15.5, 19.8),
        (12.5, 14.0),
        (18.0, 14.0),
    ]);
}

pub(super) fn mirror(p: &Pen) {
    p.dotted(&[(12.0, 2.5), (12.0, 21.5)], 2.6);
    p.closed(&[(9.5, 6.0), (9.5, 18.0), (3.5, 18.0)]);
    p.fill(&[(14.5, 6.0), (20.5, 18.0), (14.5, 18.0)]);
}

pub(super) fn symmetrize(p: &Pen) {
    p.dotted(&[(12.0, 2.5), (12.0, 21.5)], 2.6);
    octa(p, (9.5, 17.0), (3.5, 7.0));
    p.line(&[(13.5, 12.0), (19.0, 12.0)]);
    p.arrow_head((21.0, 12.0), 0.0, 3.5);
}

/// Baguette magique et étincelles (auto-rig).
pub(super) fn auto_rig(p: &Pen) {
    p.line(&[(4.0, 20.0), (14.5, 9.5)]);
    for (x, y, s) in [(18.0, 5.5, 2.5), (20.5, 12.0, 1.8), (12.5, 4.0, 1.8)] {
        p.line(&[(x - s, y), (x + s, y)]);
        p.line(&[(x, y - s), (x, y + s)]);
    }
}

/// Goutte de chaleur (carte de poids).
pub(super) fn weights(p: &Pen) {
    p.arc((12.0, 14.5), 5.5, -15.0, 195.0);
    p.line(&[(6.7, 13.1), (12.0, 3.5), (17.3, 13.1)]);
    p.dimmed()
        .fill(&[(8.0, 15.0), (16.0, 15.0), (14.5, 18.5), (9.5, 18.5)]);
}

pub(super) fn ik(p: &Pen) {
    p.line(&[(4.0, 20.0), (9.5, 11.0), (16.0, 9.0)]);
    p.dot((4.0, 20.0), 1.8);
    p.dot((9.5, 11.0), 1.8);
    p.circle((18.0, 7.0), 3.2);
    p.dot((18.0, 7.0), 1.1);
}

pub(super) fn fk(p: &Pen) {
    p.line(&[(4.0, 20.0), (11.0, 12.0), (20.0, 12.0)]);
    p.dot((4.0, 20.0), 1.8);
    p.dot((11.0, 12.0), 1.8);
    p.arc((11.0, 12.0), 5.5, -90.0, -15.0);
    p.arrow_head((11.0, 5.0), 180.0, 3.0);
}

/// Parabole de saut au-dessus du sol.
pub(super) fn ballistic(p: &Pen) {
    let arc = Pen::curve_points(3.0, 21.0, |x| 19.0 - 14.0 * (1.0 - ((x - 12.0) / 9.0).powi(2)));
    p.dotted(&arc, 2.4);
    p.dot((16.5, 8.6), 2.4);
    p.line(&[(2.0, 21.0), (22.0, 21.0)]);
}

fn figure(p: &Pen, x: f32, y: f32) {
    p.circle((x, y), 2.4);
    p.line(&[(x, y + 3.0), (x, y + 10.0)]);
    p.line(&[(x - 4.0, y + 6.0), (x + 4.0, y + 6.0)]);
    p.line(&[(x - 3.0, y + 15.0), (x, y + 10.0), (x + 3.0, y + 15.0)]);
}

pub(super) fn ghost(p: &Pen) {
    figure(&p.dimmed(), 8.0, 4.0);
    figure(p, 15.0, 5.5);
}

pub(super) fn trajectory(p: &Pen) {
    let curve = Pen::curve_points(3.5, 18.0, |x| 19.0 - 13.0 * ((x - 3.5) / 14.5).powf(0.6));
    p.dotted(&curve, 2.4);
    p.dot((3.5, 19.0), 2.0);
    p.arrow_head((20.0, 5.0), -55.0, 4.0);
}

pub(super) fn center_of_mass(p: &Pen) {
    p.circle((12.0, 12.0), 6.5);
    for (a0, a1) in [(180.0, 270.0), (0.0, 90.0)] {
        let mut pts = vec![(12.0, 12.0)];
        pts.extend(Pen::arc_points((12.0, 12.0), 6.5, a0, a1));
        p.fill(&pts);
    }
    for (a, b) in [
        ((12.0, 2.0), (12.0, 5.0)),
        ((12.0, 19.0), (12.0, 22.0)),
        ((2.0, 12.0), (5.0, 12.0)),
        ((19.0, 12.0), (22.0, 12.0)),
    ] {
        p.line(&[a, b]);
    }
}

pub(super) fn marker(p: &Pen) {
    p.circle((12.0, 12.0), 6.5);
    p.dot((12.0, 12.0), 2.6);
    for (a, b) in [
        ((12.0, 3.0), (12.0, 5.5)),
        ((12.0, 18.5), (12.0, 21.0)),
        ((3.0, 12.0), (5.5, 12.0)),
        ((18.5, 12.0), (21.0, 12.0)),
    ] {
        p.line(&[a, b]);
    }
}

pub(super) fn chain(p: &Pen) {
    p.stadium((4.5, 19.5), (11.5, 12.5), 3.0);
    p.stadium((12.5, 11.5), (19.5, 4.5), 3.0);
}

/// Sphère filaire (affichage du maillage).
pub(super) fn mesh(p: &Pen) {
    p.circle((12.0, 12.0), 8.0);
    let ell = |s: f32| move |x: f32| 12.0 + s * 3.2 * (1.0 - ((x - 12.0) / 8.0).powi(2)).max(0.0).sqrt();
    p.line(&Pen::curve_points(4.0, 20.0, ell(1.0)));
    p.dimmed().line(&Pen::curve_points(4.0, 20.0, ell(-1.0)));
    p.line(&[(12.0, 4.0), (12.0, 20.0)]);
}
