# PaintSharp Bones

**PaintSharp Bones** est une application de **rigging et d'animation 3D pour iPad**, écrite à 100 % en
Rust. Elle permet de prendre un modèle 3D, de lui générer un squelette en quelques gestes, de calculer
automatiquement ses poids de skinning, puis de le poser et de l'animer au doigt ou à l'Apple Pencil,
avant d'exporter le résultat vers Blender, Unity, Unreal ou Godot. Le moteur de rigging, le rendu
(wgpu / Metal) et l'interface (egui) sont entièrement en Rust : aucune ligne de Swift ni d'Objective-C.

## Inspirations

| Domaine | Référence | Ce qui en est repris |
| --- | --- | --- |
| Rigging | Auto-Rig Pro (extension Blender) | marqueurs « Smart », génération d'un squelette humanoïde complet, membres IK/FK avec pôle et recalage, symétrisation |
| Pose & animation | [Cascadeur](https://cascadeur.com/) | poses-clés complètes, interpolation par intervalle, points d'appui épinglés, centre de masse, trajectoires balistiques, fantômes |
| Interface | [Procreate](https://procreate.com/) & [Nomad Sculpt](https://nomadsculpt.com/) | vue 3D plein écran, blocs d'outils posés sur les bords, barre latérale à curseurs verticaux, colonne d'outils, gestes multi-doigts |

## Fonctionnalités

### Rigging

- **Auto-rig « Smart »** : des marqueurs (sommet du crâne, menton, cou, épaule, poignet, bassin,
  cheville) sont estimés automatiquement à partir de la silhouette du modèle, puis ajustables au doigt.
  Le côté droit est déduit par symétrie.
- **Génération d'un squelette humanoïde** nommé à la manière d'Auto-Rig Pro (`root.x`,
  `spine_01.x`, `arm.l`, `forearm.r`, `thigh.l`…) avec options : nombre d'os de colonne (1 à 6),
  nombre d'os de cou (1 à 3), doigts à trois phalanges, orteils.
- **Membres IK/FK** (bras et jambes) : IK analytique à deux os avec vecteur de pôle, bascule IK ⇄ FK
  avec recalage (« snap ») sur la pose courante, conservation de l'orientation de la main ou du pied.
- **Édition du squelette** : ajout d'os par glisser (extrusion depuis l'os sélectionné), déplacement
  de la queue d'un os (les enfants connectés suivent), suppression (les enfants sont rattachés au
  parent), renommage, **symétrisation** gauche → droite ou droite → gauche.

### Skinning

- **Poids par diffusion de chaleur** (« Bone Heat »,
  [Baran & Popović, SIGGRAPH 2007](https://people.csail.mit.edu/ibaran/papers/2007-SIGGRAPH-Pinocchio.pdf),
  l'algorithme des poids automatiques de Blender) : Laplacien cotangent, gradient conjugué
  préconditionné, une résolution par os en parallèle.
- **Mode enveloppes** instantané (distance aux os) pour les itérations rapides.
- Calcul **en tâche de fond** (rayon) : l'interface reste fluide pendant la liaison.
- Poids **quantifiés sur 16 octets par sommet** (4 influences, indices et poids en `u16`).
- **Skinning sur le GPU** : seules les matrices des os sont envoyées à chaque image.
- Carte de poids de l'os sélectionné (dégradé bleu → rouge).

### Pose & IK

- **Chaînes FABRIK** ([Aristidou & Lasenby, 2011](https://doi.org/10.1016/j.gmod.2011.05.003)) :
  on attrape un os et la chaîne suit, longueur réglable.
- **IK à deux os** pour les membres, avec poignées de cible et de pôle.
- **Points épinglés** (« fulcrum » de Cascadeur) : une main ou un pied reste en place pendant que l'on
  pose le reste du corps ; déplacer la racine plie les jambes sans faire glisser les pieds.
- Rotation d'un os autour de l'axe de vue, translation de la racine.
- **Pose miroir** (échange gauche/droite), réinitialisation d'un os ou de toute la pose.

### Animation & physique

- **Poses-clés complètes** sur une timeline à images, lecture en boucle, navigation de clé en clé.
- Interpolation choisie **par intervalle** : palier (*blocking*), linéaire, ou douce (Catmull-Rom).
- Modification automatique de la clé quand on retouche la pose sur une image-clé.
- **Fantômes** (*onion skin*) des clés précédente et suivante, **trajectoire** de l'os sélectionné.
- **Centre de masse** (masses déduites des poids de skinning).
- **Saut balistique** : entre une clé de décollage et une clé d'atterrissage, le centre de masse suit
  une parabole soumise à la gravité ; le résultat est cuit image par image.

### Import / export

Huit formats, avec leurs textures et matériaux quand le format en porte. Un modèle importé arrive
avec son squelette, ses poids et son animation s'il en a ; une géométrie seule se rigge ensuite dans
l'app (auto-rig, poids automatiques).

| Format | Import | Export |
| --- | --- | --- |
| `.bones` (document natif, JSON) | tout | tout (textures incluses) |
| **glTF 2.0** `.glb` / `.gltf` | maillage, UV, matériaux, textures, squelette, poids, animation | idem |
| **FBX** binaire 7.x | maillage, UV, matériaux, textures, squelette, poids, animation | — |
| **OBJ + MTL** | maillage, UV, matériaux, textures `map_Kd` | pose courante + MTL + textures |
| **STL** (ASCII / binaire) | maillage | pose courante |
| **PLY** (ASCII / binaire) | maillage, normales, UV | pose courante |
| **BVH** (capture de mouvement) | squelette, animation | squelette, animation |

- Conversion automatique des axes (Z vers le haut → Y vers le haut) et des unités (cm, mm, pouces).
- Textures externes cherchées à côté du fichier, ou intégrées (GLB, FBX) ; décodées en parallèle,
  envoyées au GPU avec leurs mipmaps, puis libérées de la mémoire du processeur.
- Vérifié sur des fichiers publics réels (modèles d'exemple Khronos *CesiumMan*, *Fox*,
  *RiggedFigure*, modèles de test d'Assimp en FBX, BVH, STL, PLY, OBJ) : import puis
  réexport dans chaque format, puis réimport.

### Interface

- Vue 3D plein écran, **blocs d'outils sombres posés sur les bords** façon Nomad Sculpt, fenêtres
  contextuelles façon Procreate, icônes vectorielles dessinées en Rust (couleurs et dimensions relevées
  sur des captures des deux applications).
- Barre latérale gauche à **curseurs verticaux** et boutons annuler / rétablir, colonne d'outils à
  droite, timeline en bas.
- Modes **Pose**, **Édition** et **Auto-rig** ; os octaédriques en « rayons X » colorés par côté.
- **Annuler / rétablir** sur les 64 dernières étapes.

### Performance

- Calculs lourds parallélisés avec **rayon** (poids, skinning CPU, trajectoires, export, physique).
- **Partage structurel** : maillage, poids et animation sont partagés par `Arc` ; l'historique
  d'annulation ne duplique que ce qui change.
- Tampons GPU réutilisés et agrandis par puissances de deux, envoi direct des tableaux du cœur sans
  copie intermédiaire, rafraîchissement uniquement quand quelque chose change.

## Architecture

Le dépôt est un *workspace* Cargo :

| Crate | Rôle |
| --- | --- |
| `crates/bones-core` | Tout le métier, sans GPU ni plateforme : entièrement testable sur poste. |
| `crates/bones-render` | Rendu wgpu (Metal sur iPad) : skinning GPU, os, poignées, grille, overlays. |
| `crates/bones-ui` | Kit d'interface egui : thème, icônes vectorielles, widgets tactiles. |
| `crates/bones-app` | Application (winit + wgpu + egui) : fenêtre, gestes, panneaux, fichiers, tâches de fond. |

Organisation de `bones-core` :

| Dossier | Contenu |
| --- | --- |
| `math/` | transformations, caméra orbitale, sélection en espace écran |
| `mesh/` | maillage indexé, UV, matériaux et textures, primitives |
| `rig/` | squelette, pose, symétrie, document `Rig` |
| `skinning/` | poids compacts, enveloppes, diffusion de chaleur, déformation |
| `ik/` | FABRIK, IK à deux os, épingles |
| `autorig/` | marqueurs, génération humanoïde, membres IK/FK |
| `anim/` | clips de poses-clés, interpolation, physique |
| `io/` | documents `.bones`, glTF, FBX, OBJ + MTL, STL, PLY, BVH (un dossier par format) |
| `editor/` | état de l'éditeur, gestes, historique, opérations |

Les détails (flux de données, révisions, passes de rendu, conventions) sont dans
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Construire et lancer

### Prérequis

- Rust **1.97** (version figée par `rust-toolchain.toml`, installée automatiquement par `rustup`),
  édition 2024.
- Pour l'iPad : macOS avec **Xcode 15 ou plus récent**, et les cibles Rust :

  ```sh
  rustup target add aarch64-apple-ios aarch64-apple-ios-sim
  ```

### Sur poste de développement (macOS, Linux)

La même application tourne en fenêtre, à la souris, ce qui accélère le développement :

```sh
cargo run -p bones-app --release
cargo test --workspace
```

### Simulateur iPad

```sh
ios/run-sim.sh            # construit, démarre un simulateur iPad, installe et lance
ios/build.sh sim release  # ou seulement construire target/ios/release/Bones.app
```

### iPad réel

```sh
export BONES_SIGN_IDENTITY="Apple Development: Prénom Nom (XXXXXXXXXX)"
export BONES_PROVISIONING_PROFILE=~/Downloads/Bones.mobileprovision
ios/build.sh device release
xcrun devicectl device install app --device <UDID> target/ios/release/Bones.app
```

L'identité se trouve avec `security find-identity -v -p codesigning` ; le profil de développement
(identifiant `com.paintsharp.bones`) se crée sur le portail Apple Developer ou via Xcode.

## Gestes

| Geste | Action |
| --- | --- |
| Un doigt sur un os | manipuler selon l'outil (IK, rotation, épingle, queue, nouvel os) |
| Un doigt dans le vide | tourner autour du modèle |
| Toucher le vide | désélectionner |
| Pincer à deux doigts | zoomer |
| Glisser à deux doigts | déplacer la vue |
| Toucher à deux doigts | annuler |
| Toucher à trois doigts | rétablir |

Sur poste : clic gauche (manipuler / tourner), clic droit ou du milieu (déplacer), molette (zoomer).

Raccourcis clavier (ordinateur ou Magic Keyboard) : `⌘Z` / `Ctrl+Z` annuler, `⇧⌘Z` rétablir,
`Espace` lecture, `K` poser une clé, `←` / `→` image précédente / suivante, `F` cadrer,
`1` / `2` / `3` modes Pose / Édition / Auto-Rig, `Suppr` supprimer l'os (mode Édition),
`Échap` fermer la fenêtre contextuelle.

## Fichiers

Sur iPad, les documents `.bones`, les modèles `.obj` à importer et les exports `.glb` se trouvent
dans le dossier **Documents** de l'application, visible dans l'app **Fichiers** sous
« Sur mon iPad › Bones ». Il suffit d'y déposer un `.obj` pour le retrouver dans la galerie.
Sur poste de développement, c'est le dossier `~/Documents/PaintSharp Bones` (ou celui indiqué par la
variable d'environnement `BONES_DOCUMENTS`). La session en cours est enregistrée automatiquement dans
`Session.bones` à la fermeture ou au passage en arrière-plan, et rouverte au lancement.

## Feuille de route

Ce qui **n'est pas encore** fait :

- AutoPosing par réseau de neurones comme Cascadeur (l'actuel repose sur l'IK et les épingles).
- AutoPhysics complète : corps rigides, ragdoll, moment cinétique, correction de l'équilibre.
- Test de visibilité dans la diffusion de chaleur (segment sommet–os contenu dans le volume).
- Pinceau de peinture de poids, lissage et normalisation manuels.
- Rig facial, os de torsion (*twist*), contrôleurs personnalisés, sélecteur d'os (*picker*).
- Reciblage d'animation (*remap*) entre squelettes, import Mixamo.
- Export FBX, import FBX ASCII, Collada (`.dae`), USD(Z) ; cibles de morphose (*blend shapes*),
  cartes de normales / rugosité, second jeu d'UV.
- Capture de mouvement à partir de vidéo.
- Sélecteur de documents de l'app Fichiers (UIDocumentPicker) et ouverture « sur place ».
- Icônes, captures et métadonnées pour l'App Store.
- Localisation anglaise.

## Licence

À définir par l'auteur du dépôt.
