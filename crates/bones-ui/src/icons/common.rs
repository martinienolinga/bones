//! Icônes génériques : fichiers, historique, lecture, réglages.

use super::pen::Pen;

pub(super) fn gallery(p: &Pen) {
    for (x, y) in [(4.0, 4.0), (13.0, 4.0), (4.0, 13.0), (13.0, 13.0)] {
        p.rect((x, y), (x + 7.0, y + 7.0), 1.5);
    }
}

/// Clé à molette, comme le menu « Actions » de Procreate.
pub(super) fn actions(p: &Pen) {
    p.line(&[(4.5, 19.5), (12.5, 11.5)]);
    p.arc((15.5, 8.5), 4.5, 75.0, 375.0);
}

pub(super) fn adjust(p: &Pen) {
    for (y, x) in [(7.0, 9.0), (12.0, 15.0), (17.0, 7.0)] {
        p.line(&[(4.0, y), (20.0, y)]);
        p.dot((x, y), 2.2);
    }
}

pub(super) fn delete(p: &Pen) {
    p.line(&[(4.5, 7.0), (19.5, 7.0)]);
    p.line(&[(9.5, 7.0), (9.5, 4.5), (14.5, 4.5), (14.5, 7.0)]);
    p.line(&[(6.5, 7.0), (7.5, 20.0), (16.5, 20.0), (17.5, 7.0)]);
    p.line(&[(10.5, 10.5), (10.5, 16.5)]);
    p.line(&[(13.5, 10.5), (13.5, 16.5)]);
}

/// Flèche courbe ; `redo` place la pointe à droite.
pub(super) fn history(p: &Pen, redo: bool) {
    p.arc((12.0, 15.0), 7.0, 180.0, 360.0);
    if redo {
        p.arrow_head((19.0, 16.5), 90.0, 4.0);
    } else {
        p.arrow_head((5.0, 16.5), 90.0, 4.0);
    }
}

pub(super) fn play(p: &Pen) {
    p.fill(&[(8.0, 5.0), (19.0, 12.0), (8.0, 19.0)]);
}

pub(super) fn pause(p: &Pen) {
    p.rect_filled((7.0, 5.0), (10.5, 19.0), 1.0);
    p.rect_filled((13.5, 5.0), (17.0, 19.0), 1.0);
}

/// Saut à la clé voisine ; `next` vers la droite.
pub(super) fn jump_key(p: &Pen, next: bool) {
    if next {
        p.fill(&[(6.0, 6.0), (16.0, 12.0), (6.0, 18.0)]);
        p.line(&[(18.0, 6.0), (18.0, 18.0)]);
    } else {
        p.fill(&[(18.0, 6.0), (18.0, 18.0), (8.0, 12.0)]);
        p.line(&[(6.0, 6.0), (6.0, 18.0)]);
    }
}

pub(super) fn key(p: &Pen) {
    p.closed(&[(12.0, 3.5), (20.5, 12.0), (12.0, 20.5), (3.5, 12.0)]);
    p.fill(&[(12.0, 8.0), (16.0, 12.0), (12.0, 16.0), (8.0, 12.0)]);
}

pub(super) fn key_remove(p: &Pen) {
    p.closed(&[(12.0, 3.5), (20.5, 12.0), (12.0, 20.5), (3.5, 12.0)]);
    p.line(&[(8.0, 12.0), (16.0, 12.0)]);
}

pub(super) fn eye(p: &Pen) {
    let lid = |s: f32| move |x: f32| 12.0 + s * 6.5 * (1.0 - ((x - 12.0) / 9.0).powi(2));
    p.line(&Pen::curve_points(3.0, 21.0, lid(-1.0)));
    p.line(&Pen::curve_points(3.0, 21.0, lid(1.0)));
    p.circle((12.0, 12.0), 3.0);
}

pub(super) fn export(p: &Pen) {
    p.line(&[
        (9.0, 10.0),
        (5.5, 10.0),
        (5.5, 20.5),
        (18.5, 20.5),
        (18.5, 10.0),
        (15.0, 10.0),
    ]);
    p.line(&[(12.0, 15.0), (12.0, 4.5)]);
    p.arrow_head((12.0, 3.0), -90.0, 4.0);
}

pub(super) fn import(p: &Pen) {
    p.line(&[(5.0, 15.0), (5.0, 20.0), (19.0, 20.0), (19.0, 15.0)]);
    p.line(&[(12.0, 3.5), (12.0, 13.5)]);
    p.arrow_head((12.0, 15.5), 90.0, 4.0);
}

pub(super) fn save(p: &Pen) {
    p.closed(&[(5.0, 4.0), (16.0, 4.0), (20.0, 8.0), (20.0, 20.0), (5.0, 20.0)]);
    p.rect((8.5, 4.0), (15.0, 8.5), 0.5);
    p.rect((8.0, 13.0), (17.0, 20.0), 0.5);
}

pub(super) fn folder(p: &Pen) {
    p.closed(&[
        (3.0, 6.5),
        (9.5, 6.5),
        (11.5, 9.0),
        (21.0, 9.0),
        (21.0, 19.0),
        (3.0, 19.0),
    ]);
}

pub(super) fn settings(p: &Pen) {
    p.circle((12.0, 12.0), 3.2);
    p.circle((12.0, 12.0), 6.5);
    for i in 0..8 {
        let a = (i as f32 * 45.0).to_radians();
        let (c, s) = (a.cos(), a.sin());
        p.line(&[(12.0 + 6.5 * c, 12.0 + 6.5 * s), (12.0 + 9.0 * c, 12.0 + 9.0 * s)]);
    }
}

pub(super) fn close(p: &Pen) {
    p.line(&[(6.0, 6.0), (18.0, 18.0)]);
    p.line(&[(18.0, 6.0), (6.0, 18.0)]);
}

pub(super) fn check(p: &Pen) {
    p.line(&[(5.0, 12.5), (10.0, 17.5), (19.0, 7.0)]);
}

/// Crochets de cadrage avec point central.
pub(super) fn frame(p: &Pen) {
    p.line(&[(4.0, 9.0), (4.0, 4.0), (9.0, 4.0)]);
    p.line(&[(15.0, 4.0), (20.0, 4.0), (20.0, 9.0)]);
    p.line(&[(20.0, 15.0), (20.0, 20.0), (15.0, 20.0)]);
    p.line(&[(9.0, 20.0), (4.0, 20.0), (4.0, 15.0)]);
    p.dot((12.0, 12.0), 2.0);
}

pub(super) fn grid(p: &Pen) {
    p.rect((4.0, 4.0), (20.0, 20.0), 1.0);
    for t in [9.33, 14.66] {
        p.line(&[(t, 4.0), (t, 20.0)]);
        p.line(&[(4.0, t), (20.0, t)]);
    }
}

pub(super) fn hierarchy(p: &Pen) {
    p.rect((9.0, 3.0), (15.0, 8.0), 1.0);
    p.line(&[(12.0, 8.0), (12.0, 11.0)]);
    p.line(&[(6.0, 14.0), (6.0, 11.0), (18.0, 11.0), (18.0, 14.0)]);
    p.rect((3.0, 14.0), (9.0, 19.5), 1.0);
    p.rect((15.0, 14.0), (21.0, 19.5), 1.0);
}
