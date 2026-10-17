//! Soudure des sommets confondus.
//!
//! Les formats texturés dupliquent les sommets le long des coutures UV ou
//! des arêtes vives. Pour le calcul des poids, ces doublons doivent être
//! traités comme un seul point, sinon la peau se fendrait aux coutures.

use std::collections::HashMap;

use super::Mesh;

impl Mesh {
    /// Pour chaque sommet, l'indice de son représentant (premier sommet de
    /// même position exacte), et le nombre de représentants distincts.
    pub fn weld_map(&self) -> (Vec<u32>, usize) {
        let mut first: HashMap<[u32; 3], u32> = HashMap::with_capacity(self.positions.len());
        let mut map = Vec::with_capacity(self.positions.len());
        let mut unique = 0;
        for (i, p) in self.positions.iter().enumerate() {
            let key = [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()];
            let rep = *first.entry(key).or_insert_with(|| {
                unique += 1;
                i as u32
            });
            map.push(rep);
        }
        (map, unique)
    }

    /// Vrai si des sommets partagent la même position.
    pub fn has_seams(&self) -> bool {
        self.weld_map().1 < self.positions.len()
    }
}
