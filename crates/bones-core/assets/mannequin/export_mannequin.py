# Regénère mannequin.glb depuis le bundle « Human Base Meshes » (Blender Studio, CC0).
#
#   blender -b --factory-startup human_base_meshes_bundle.blend \
#       --python export_mannequin.py -- mannequin.glb
#
# Testé avec Blender 4.5 LTS et le bundle v1.4.1. Étapes : corps masculin
# réaliste (« Body Male - Realistic ») et ses deux yeux, maillage de base du
# multirésolution (niveau 0, sans les détails sculptés), fusion, soudure,
# triangulation, mise à l'échelle (1,75 m), pieds en y = 0, centré en X et Z,
# lissage, sans UV ni matériau (argile par défaut de l'app). L'exportateur
# glTF convertit le repère de Blender (Z haut, regard -Y) en Y haut, regard +Z.
import sys

import bmesh
import bpy
from mathutils import Vector

out = sys.argv[sys.argv.index("--") + 1]
HEIGHT = 1.75

body = bpy.data.objects["GEO-body_male_realistic"]
keep = [body, bpy.data.objects["GEO-body_male_realistic.eye.L"], bpy.data.objects["GEO-body_male_realistic.eye.R"]]
scene = bpy.context.scene
for ob in list(bpy.data.objects):
    if ob not in keep:
        bpy.data.objects.remove(ob, do_unlink=True)
for ob in keep:
    if ob.name not in scene.collection.objects:
        scene.collection.objects.link(ob)
bpy.context.view_layer.update()


def activate(ob):
    for o in bpy.context.view_layer.objects:
        o.select_set(False)
    ob.select_set(True)
    bpy.context.view_layer.objects.active = ob


# Niveau 0 du multirésolution : ses sommets sont déjà sur la surface sculptée.
activate(body)
for mod in [m for m in body.modifiers if m.type == "MULTIRES"]:
    bpy.ops.object.modifier_remove(modifier=mod.name)

for ob in keep:
    activate(ob)
    bpy.ops.object.parent_clear(type="CLEAR_KEEP_TRANSFORM")
for ob in keep:
    ob.select_set(True)
bpy.context.view_layer.objects.active = body
bpy.ops.object.join()
activate(body)
bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)

me = body.data
bm = bmesh.new()
bm.from_mesh(me)
bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-5)
bmesh.ops.triangulate(bm, faces=bm.faces, quad_method="BEAUTY", ngon_method="BEAUTY")
lo = Vector([min(v.co[i] for v in bm.verts) for i in range(3)])
hi = Vector([max(v.co[i] for v in bm.verts) for i in range(3)])
scale = HEIGHT / (hi.z - lo.z)
offset = Vector(((lo.x + hi.x) * 0.5, (lo.y + hi.y) * 0.5, lo.z))
for v in bm.verts:
    v.co = (v.co - offset) * scale
bm.to_mesh(me)
bm.free()
for p in me.polygons:
    p.use_smooth = True
while me.uv_layers:
    me.uv_layers.remove(me.uv_layers[0])
me.materials.clear()
for name in [a.name for a in me.attributes if not a.name.startswith(".") and a.name != "position"]:
    try:
        me.attributes.remove(me.attributes[name])
    except RuntimeError:
        pass  # attribut interne non supprimable
me.update()
body.name = me.name = "Mannequin"
print("sommets", len(me.vertices), "triangles", len(me.polygons), "échelle", scale)

activate(body)
bpy.ops.export_scene.gltf(
    filepath=out,
    export_format="GLB",
    use_selection=True,
    export_yup=True,
    export_apply=True,
    export_normals=True,
    export_materials="NONE",
    export_skins=False,
    export_animations=False,
    export_morph=False,
    export_extras=False,
)
