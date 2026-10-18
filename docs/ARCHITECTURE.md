# Architecture de PaintSharp Bones

Ce document décrit comment les données circulent dans l'application, les principes de performance
et de mémoire, les conventions du projet, et la marche à suivre pour ajouter un outil.

## Vue d'ensemble

```
            gestes (doigts, Pencil, souris)
                        │
                        ▼
┌──────────────────────────────────────────────┐
│ bones-app                                    │
│  winit (événements) → reconnaissance gestes  │
│  egui + bones-ui (panneaux, outils)          │
│  tâches de fond (liaison, fichiers, export)  │
└───────────────┬──────────────────────────────┘
                │ appels de l'API Editor
                ▼
┌──────────────────────────────────────────────┐
│ bones-core                                   │
│  Editor : modes, outils, gestes, historique  │
│   └─ Rig (document)                          │
│       skeleton, pose, limbs, pins            │
│       mesh: Arc<Mesh>  skin: Arc<Skin>       │
│       clip: Arc<Clip>  revisions             │
└───────────────┬──────────────────────────────┘
                │ matrices, instances, révisions
                ▼
┌──────────────────────────────────────────────┐
│ bones-render (wgpu → Metal)                  │
│  tampons statiques : positions, normales,    │
│  poids, indices (renvoyés si révision change)│
│  par image : matrices d'os, instances, lignes│
└──────────────────────────────────────────────┘
```

`bones-core` ne connaît ni le GPU ni la plateforme : toute la logique (IK, skinning, auto-rig,
animation, gestes de l'éditeur) se teste avec `cargo test` sur un poste quelconque. L'application
iPad se contente de traduire les événements en appels à l'API de `Editor`, puis de transmettre au
moteur de rendu ce qu'il faut dessiner.

## Flux de données

### Gestes → éditeur

1. winit fournit les touchers (avec force et inclinaison pour l'Apple Pencil) ou la souris.
2. Le module de gestes de l'app décide si le toucher appartient à l'interface egui (il commence sur
   un panneau) ou à la vue 3D.
3. Dans la vue 3D, un premier doigt est proposé à l'éditeur : `Editor::pointer_down` retourne `true`
   s'il touche un os, une poignée IK ou un marqueur, et l'éditeur suit alors le geste
   (`pointer_move`, `pointer_up`). Sinon le geste pilote la caméra (orbite).
4. Un second doigt annule la manipulation en cours (`pointer_cancel`) et passe en zoom / panoramique.
   Un toucher bref à deux doigts annule, à trois doigts rétablit.

Toutes les manipulations travaillent dans le plan parallèle à l'écran passant par l'élément saisi,
avec un décalage mémorisé au début du geste : l'os ne « saute » jamais sous le doigt.

### Document `Rig` et partage structurel

Le document regroupe le squelette, la pose, les membres IK/FK, les épingles et trois données
lourdes partagées par `Arc` : le maillage, les poids et l'animation. Cloner un `Rig` (ce que fait
l'historique à chaque étape) ne copie que le squelette et la pose ; une donnée lourde n'est dupliquée
qu'au moment où elle est modifiée (`Arc::make_mut`) ou remplacée.

La pose est la **seule source de vérité** : les cibles et pôles IK ne sont que des poignées,
recalées sur la pose quand on change d'image. Les clés d'animation stockent donc des poses complètes
(rotations « cuites »), lisibles par n'importe quel moteur.

### Objets de la scène (module `scene`)

Le maillage principal reste le **corps** (`rig.mesh` / `rig.skin`) : l'auto-rig, la peinture de
poids, le skinning et les préréglages le ciblent toujours. Autour de lui, `rig.objects()` liste des
**objets supplémentaires** (`SceneObject`), à la manière de Blender : accessoires (épée, chapeau,
lunettes), vêtements, parties du corps séparées. Un objet a un identifiant stable et sérialisé
(`ObjectId`), un nom, un maillage partagé par `Arc`, un parent, une transformation locale, une
liaison, un drapeau de visibilité et de verrou, et ses propres révisions.

- **Parent** (`Parent::{World, Object(ObjectId), Bone(BoneId)}`) : la transformation locale est
  relative au parent ; pour un os, relative à sa transformation monde **posée** (l'objet suit l'os).
  Les cycles sont refusés. Le parentage d'os survit aux modifications du squelette via le même
  `BoneRemap` que les poids : si l'os disparaît, l'objet passe à l'os héritier (ou au monde) en
  gardant sa place ; au remplacement complet du squelette (auto-rig), les os sont retrouvés par
  leur nom.
- **Liaison** (`Binding::{Rigid, Skinned{SkinBinding}}`) : `Rigid` suit rigidement le parent ;
  `Skinned` est déformé par l'armature avec ses propres poids. Les méthodes de poids
  (`SkinMethod::{Transfer, Heat, Envelope, Manual}`) sont calculées en tâche de fond par la **même
  file que le corps** (`Editor::bind_job` / `finish_bind`, cf. « Liaison en tâche de fond »). Le
  transfert copie les poids du corps au point le plus proche (« Data Transfer » de Blender,
  `skinning/closest.rs` + `transfer.rs`) ; un objet déformé est d'abord placé par sa transformation
  évaluée **au repos** (placement de liaison), puis déformé par la palette d'os du corps.
- Cloner un `Rig` reste peu coûteux : les maillages et poids des objets sont partagés par `Arc`,
  donc l'historique d'annulation ne les copie pas.
- Les transformations des objets **ne sont pas animées** pour l'instant (un accessoire s'anime en
  suivant l'os auquel il est attaché) ; extension possible.

### Révisions

Chaque donnée porte un numéro de révision unique (compteur atomique global, jamais réutilisé) :
squelette, maillage, poids, animation, et le couple (squelette, maillage) pour lequel les poids ont
été calculés. Ces numéros ne sont pas sérialisés.

- Le moteur de rendu compare les révisions du maillage et des poids à celles de ses tampons GPU et ne
  renvoie les données que si elles ont changé (après un import, une liaison, une annulation…).
- La trajectoire de l'os sélectionné et les masses du centre de masse sont mises en cache de la même
  façon.
- `Rig::needs_bind` indique que les poids ne correspondent plus au squelette ou au maillage.

### Liaison en tâche de fond

Quand le squelette change (fin d'un geste d'édition, auto-rig, import), `Editor::bind_job` fournit
une tâche autonome (maillage partagé, copie du squelette, méthode, révisions visées). L'app l'exécute
sur un fil de fond ; la diffusion de chaleur parallélise elle-même ses résolutions avec rayon. Le
résultat revient par un canal et `Editor::finish_bind` ne l'installe que si les révisions visées
sont toujours d'actualité : un résultat périmé (squelette modifié ou annulation entre-temps) est
simplement ignoré.

### Projets `.kbrig`

Chaque projet est un fichier `.kbrig` autonome (`bones_core::project`) : un conteneur ZIP (deflate
pur Rust) avec un manifeste, une vignette JPEG stockée sans recompression (la galerie la lit sans
rien décompresser d'autre), le `Rig` en JSON allégé de ses données lourdes, le maillage, les poids
et les poses des clés en binaire, et les textures telles quelles. `ProjectSnapshot::capture` ne
copie que des `Arc` : l'enregistrement se fait sur un fil de fond, de façon atomique (fichier
temporaire puis renommage). Les fichiers reçus d'ailleurs sont lus avec des tailles bornées et
validés ; un fichier d'une version plus récente est refusé. `project::container` et
`project::store` (noms, renommage, duplication) servent aussi aux autres formats de l'app.

### Rendu

Deux passes par image, puis l'interface :

1. **Scène** : effacement couleur + profondeur, maillage skinné sur le GPU (les poids sont un tampon
   de sommets de 16 octets par sommet, les matrices d'os un tampon de stockage), puis la grille
   testée en profondeur.
2. **Rayons X** : la profondeur est effacée, puis os octaédriques, fantômes et poignées (instanciés),
   et lignes d'overlay (trajectoires, pôles IK, centre de masse) sans test de profondeur. Les os
   restent ainsi visibles à travers le maillage, comme dans les logiciels de rigging.
3. **Interface** egui, chargée par-dessus.

L'éclairage est calculé en linéaire et converti en sRGB dans le shader, car la surface utilise un
format non sRGB (celui que préfère egui).

Les **vignettes des galeries de préréglages** montrent le mannequin qui joue le préréglage. Un fil
de fond le cuit sur le mannequin (`presets::Staged` : animation reciblée sur place, ou expression
à pleine intensité, cadrage calculé sur des sommets sondes déformés). Chaque carte visible demande
sa vignette par les données du contexte egui ; après l'interface, `bones-app/src/showcase` la rend
avec un moteur dédié (maillage du mannequin) dans sa propre texture native d'egui
(`bones_render::Tiles`, fond transparent, aucune relecture). Une expression n'est rendue qu'une
fois ; une animation à chaque image de son cycle (30 i/s au plus).

## Performance et mémoire

- **GPU d'abord** : le skinning se fait dans le vertex shader ; par image, seules les matrices d'os
  (64 octets par os), les instances et les lignes sont envoyées.
- **Pas de copie inutile** : positions, normales et poids sont envoyés directement depuis les
  tableaux du cœur (`bytemuck`), sans format intermédiaire.
- **Tampons réutilisés** : les tampons dynamiques grandissent par puissances de deux et ne sont
  jamais réalloués à chaque image.
- **Parallélisme** : rayon pour les poids (un os par tâche en diffusion de chaleur, un sommet par
  tâche en enveloppes), le skinning CPU (export), l'échantillonnage des trajectoires, l'export glTF
  et la cuisson balistique.
- **Données compactes** : poids quantifiés en `u16` (somme exactement égale à 1), tableaux du
  maillage réduits à leur taille (`shrink_to_fit`) après import.
- **Historique borné** à 64 étapes, à coût marginal grâce au partage par `Arc`.
- **Rafraîchissement à la demande** : la boucle d'événements attend ; une image n'est produite que
  lors d'une interaction, d'une lecture d'animation ou d'une demande d'egui.
- Les dépendances sont compilées optimisées même en debug (`[profile.dev.package."*"]`), et la
  version release utilise la LTO.

## Conventions

- **Repère** : main droite, **Y vers le haut**, le personnage regarde vers **+Z** ; son côté
  gauche est en **+X**. Une unité vaut un mètre (gravité 9,81).
- **Os** : un os pointe le long de son axe **+Y local** ; la tête est à l'origine de l'os, la queue
  en `(0, longueur, 0)`. La transformation de repos est relative à l'espace du parent.
- **Ordre** : dans un `Skeleton`, le parent a toujours un indice inférieur à ses enfants ; la
  cinématique directe est une simple boucle.
- **Pose** : delta appliqué *après* la transformation de repos (comme le mode pose de Blender) ;
  l'identité partout correspond à la pose de repos.
- **Noms** : suffixes à la Auto-Rig Pro, `.l` / `.r` pour les côtés et `.x` pour l'axe central
  (`.L`/`.R`, `_l`/`_r`, `Left`/`Right` sont aussi reconnus pour la symétrie).
- **Caméra** : projection perspective NDC Z ∈ [0, 1], Y vers le haut (convention wgpu / Metal).
- **Code** : identifiants en anglais, commentaires et messages en français ; fichiers courts,
  regroupés par thème ; tests unitaires dans des fichiers `tests.rs` séparés.

## Ajouter un outil

Exemple : un outil de pose « Échelle ».

1. **Cœur** (`bones-core/src/editor/`) :
   - ajouter la variante à `PoseTool` ;
   - dans la gestion du `pointer_down` du mode Pose, créer une nouvelle variante de `Drag` qui
     mémorise l'état de départ (pose de l'os, point saisi) ;
   - dans `pointer_move`, appliquer la transformation puis appeler le maintien des contraintes
     (membres IK et épingles) en verrouillant les os manipulés ;
   - `pointer_up` met à jour la clé courante s'il y en a une ;
   - écrire un test dans `editor/tests.rs` qui simule le geste sur `Rig::demo()` avec une caméra de
     face, et vérifie le résultat ainsi que l'annulation.
2. **Icône** (`bones-ui`) : ajouter une variante à l'énumération des icônes et son dessin
   vectoriel.
3. **Interface** (`bones-app`) : ajouter le bouton dans la colonne d'outils du mode Pose.

Un outil qui modifie le squelette de repos doit appeler `Rig::touch_skeleton()` : la liaison en
tâche de fond sera relancée automatiquement à la fin du geste. Un outil qui modifie l'animation
passe par `Rig::clip_mut()`, qui met à jour la révision de l'animation.

## API des objets de scène (pour les phases 2 et 3)

Le cœur (`bones-core`, étape 1) expose tout ce qu'il faut au rendu (étape 2, `bones-render` +
construction de scène) et à l'interface (étape 3, nouvel outliner + mode Objet). Rien n'est à
ajouter au cœur pour ces étapes ; les signatures ci-dessous sont stables.

### Rendu (`Editor`, lecture seule)

- `Editor::body_visible() -> bool` : dessiner le corps (en plus de `ViewOptions::mesh`).
- `Editor::object_draws() -> Vec<ObjectDraw>` : un appel par objet **visible**. Chaque `ObjectDraw`
  porte `id: ObjectId`, `mesh: Arc<Mesh>`, `mesh_revision: u64` (clé des tampons GPU, à coupler avec
  `id`), `model: Mat4`, `skin: Option<SkinDraw>`, `selected: bool`.
  - Objet **rigide** (`skin == None`) : dessiner `mesh` avec la matrice `model`, sans palette d'os.
  - Objet **déformé** (`skin == Some`) : `model` est le **placement de liaison** ; appliquer `model`
    puis le skinning linéaire avec `Editor::skinning_matrices()` (la **même** palette que le corps)
    et les poids `SkinDraw { skin: Arc<Skin>, revision }`. Les sommets sans poids gardent leur
    position (comme le corps). Le shader `mesh.wgsl` convient tel quel (poids `u16`, matrices d'os).
- Révisions : `ObjectDraw::mesh_revision` et `SkinDraw::revision` changent quand la géométrie ou les
  poids de l'objet changent ; les caches GPU se clavent sur `(id, mesh_revision)` et
  `(id, skin.revision)`. Surbrillance de la sélection : `ObjectDraw::selected`.

### Interface (`Editor`)

- Liste : `Editor::outliner() -> Vec<OutlinerRow>` (corps en premier, puis objets en profondeur
  d'abord). `OutlinerRow { item: ObjectRef, name, depth, parent, skinned, visible, locked,
  children }`. `Editor::object_info(id) -> Option<ObjectInfo>` pour le panneau de propriétés.
- Sélection (hors historique) : champ `Editor::selected_object: Option<ObjectRef>`,
  `select_object`, `select_object_parent` / `_child` / `next_object_sibling`, `selected_extra`,
  `frame_selected_object`. `ObjectRef::{Body, Extra(ObjectId)}`.
- Mode : `Mode::Object` (nouvelle variante). Dans ce mode, `pointer_down/move/up` sélectionnent et
  déplacent l'objet sous le doigt ; le manipulateur 3D (`manipulator_*`) agit sur l'objet
  sélectionné (ou sur le modèle entier pour le corps). Chaque glissement produit **une** étape
  d'annulation. `pick_object(pos, vp) -> Option<ObjectRef>` fait le lancer de rayon (objets
  déformés piqués sur leur position déformée).
- Opérations (toutes passent par l'historique, renvoient `Result<_, ObjectError>` avec message
  anglais) : `add_primitive(Primitive)`, `add_object_mesh(name, Mesh)`,
  `add_imported_objects(io::SceneImport)`, `duplicate_object`, `delete_object` /
  `delete_selected_object`, `rename_object`, `set_object_visible`, `set_object_locked`,
  `set_object_parent(id, Parent, keep_transform)`, `parent_object_to_selected_bone`,
  `clear_object_parent`, `set_object_binding(id, Option<SkinMethod>)`, `rebind_object`,
  `set_object_transform(id, Transform)` (regroupe les retouches en une étape), `reorder_object`.
- Primitives : `mesh::Primitive::{Cube, Sphere, Cylinder, Cone, Plane, Torus}` (`ALL`, `label()`,
  `mesh()`).
- Liaison en tâche de fond : `Editor::bind_job()` renvoie désormais aussi les tâches des objets
  déformés (champ `BindJob::object`) ; `finish_bind` route le résultat. `objects_binding()` indique
  qu'un calcul d'objet est en attente.

### Import en objets (`io`)

- `io::import_scene(path)` / `import_scene_bytes(format, bytes, name, base)` → `SceneImport`
  (un objet par maillage, hiérarchie conservée). `SceneImport::into_rig()` crée un projet (le plus
  grand maillage skinné devient le corps) ; `Editor::add_imported_objects` ajoute au projet ouvert
  (os retrouvés par leur nom). L'import « personnage » classique (`io::import`) reste inchangé : il
  fusionne tout dans le corps.
