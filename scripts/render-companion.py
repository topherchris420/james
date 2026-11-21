"""Rebuild the bundled octopus artwork with Blender 4.3 and Pillow.

blender -b -t 6 --python scripts/render-companion.py
Pass `-- rest 0` (or watch/bell and a frame from 0 to 11) for a single proof.
The app plays the finished contact sheets; Blender is only an artist's tool.
"""

import math
import sys
import tempfile
from pathlib import Path

import bpy
from mathutils import Vector
from bpy_extras.object_utils import world_to_camera_view
from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
DEST = ROOT / "crates/james-desktop/assets/octopus"
WIDTH, HEIGHT = 448, 304
COLUMNS, ROWS = 4, 3
STATES = {"rest": "9b72c3", "watch": "4998af", "bell": "d57869"}

# Consecutive points travel from the web to the tip. The arms sit at different
# depths, with three foreshortened behind the mantle and two crossing the front.
ARMS = [
    [
        (-0.32, 0.22, 0.45),
        (-0.68, 0.42, 0.31),
        (-1.04, 0.63, 0.32),
        (-1.23, 0.72, 0.56),
        (-1.15, 0.73, 0.72),
        (-1.00, 0.71, 0.68),
    ],
    [
        (-0.10, 0.28, 0.44),
        (-0.26, 0.63, 0.26),
        (-0.41, 1.02, 0.23),
        (-0.70, 1.30, 0.36),
        (-0.70, 1.35, 0.57),
        (-0.54, 1.30, 0.62),
    ],
    [
        (0.18, 0.32, 0.45),
        (0.45, 0.65, 0.29),
        (0.64, 0.99, 0.27),
        (0.92, 1.17, 0.38),
        (1.08, 1.09, 0.61),
        (0.96, 0.97, 0.71),
    ],
    [
        (0.36, 0.10, 0.45),
        (0.69, 0.38, 0.30),
        (1.05, 0.60, 0.29),
        (1.34, 0.69, 0.39),
        (1.50, 0.66, 0.60),
        (1.54, 0.59, 0.74),
    ],
    [
        (-0.42, -0.04, 0.49),
        (-0.80, -0.12, 0.32),
        (-1.11, -0.20, 0.28),
        (-1.37, -0.25, 0.42),
        (-1.53, -0.25, 0.66),
        (-1.53, -0.27, 0.85),
        (-1.40, -0.30, 0.95),
        (-1.26, -0.34, 0.89),
        (-1.26, -0.37, 0.78),
    ],
    [
        (-0.27, -0.28, 0.46),
        (-0.46, -0.67, 0.28),
        (-0.60, -0.95, 0.20),
        (-0.82, -1.18, 0.18),
        (-1.11, -1.27, 0.17),
        (-1.34, -1.19, 0.22),
        (-1.42, -1.07, 0.32),
        (-1.37, -0.94, 0.42),
    ],
    [
        (0.07, -0.34, 0.46),
        (0.16, -0.71, 0.32),
        (0.38, -0.90, 0.25),
        (0.56, -1.13, 0.20),
        (0.59, -1.34, 0.16),
        (0.44, -1.54, 0.14),
        (0.18, -1.68, 0.12),
        (-0.05, -1.64, 0.15),
        (-0.14, -1.54, 0.21),
    ],
    [
        (0.36, -0.18, 0.48),
        (0.74, -0.43, 0.32),
        (1.10, -0.53, 0.26),
        (1.43, -0.49, 0.37),
        (1.59, -0.45, 0.62),
        (1.75, -0.41, 0.73),
        (1.88, -0.43, 0.64),
    ],
]


def linear(hex_color):
    def channel(pair):
        s = int(pair, 16) / 255
        return s / 12.92 if s <= 0.04045 else ((s + 0.055) / 1.055) ** 2.4

    return (*[channel(hex_color[i : i + 2]) for i in (0, 2, 4)], 1)


def material(name, color, roughness, metal=0, coat=0):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    shader = m.node_tree.nodes.get("Principled BSDF")
    for key, value in {
        "Base Color": linear(color),
        "Roughness": roughness,
        "Metallic": metal,
        "Coat Weight": coat,
        "Coat Roughness": 0.24,
    }.items():
        shader.inputs[key].default_value = value
    return m, shader


def clay(mode):
    skin, shader = material("Satin glaze", STATES[mode], 0.37, coat=0.23)
    tree = skin.node_tree
    noise = tree.nodes.new("ShaderNodeTexNoise")
    noise.inputs["Scale"].default_value = 190
    noise.inputs["Detail"].default_value = 2
    bump = tree.nodes.new("ShaderNodeBump")
    bump.inputs["Strength"].default_value = 0.15
    bump.inputs["Distance"].default_value = 0.006
    tree.links.new(noise.outputs["Fac"], bump.inputs["Height"])
    tree.links.new(bump.outputs["Normal"], shader.inputs["Normal"])
    # Brush a darker underglaze into the web. The highlight belongs to the glaze,
    # so it travels continuously over both the tinted and untinted porcelain.
    coords = tree.nodes.new("ShaderNodeTexCoord")
    axes = tree.nodes.new("ShaderNodeSeparateXYZ")
    tree.links.new(coords.outputs["Generated"], axes.inputs[0])
    wash = tree.nodes.new("ShaderNodeValToRGB")
    wash.color_ramp.elements[0].position = 0.04
    wash.color_ramp.elements[0].color = linear(
        {"rest": "71548c", "watch": "376c81", "bell": "a85c57"}[mode]
    )
    wash.color_ramp.elements[1].position = 0.77
    wash.color_ramp.elements[1].color = linear(STATES[mode])
    tree.links.new(axes.outputs["Z"], wash.inputs[0])
    tree.links.new(wash.outputs["Color"], shader.inputs["Base Color"])
    return skin


def ellipsoid(name, center, radii, mat, segments=40):
    bpy.ops.mesh.primitive_uv_sphere_add(
        segments=segments, ring_count=segments // 2, location=center
    )
    obj = bpy.context.object
    obj.name, obj.scale = name, radii
    obj.data.materials.append(mat)
    for face in obj.data.polygons:
        face.use_smooth = True
    return obj


def surface(name, rings, mat):
    n = len(rings[0])
    faces = [
        (k * n + i, k * n + (i + 1) % n, (k + 1) * n + (i + 1) % n, (k + 1) * n + i)
        for k in range(len(rings) - 1)
        for i in range(n)
    ]
    faces.extend([tuple(reversed(range(n))), tuple(range(n * (len(rings) - 1), n * len(rings)))])
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata([v for ring in rings for v in ring], [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    obj.data.materials.append(mat)
    for face in mesh.polygons:
        face.use_smooth = True
    return obj


def radial(profile, x, y, mat, name, y_scale=1):
    return surface(
        name,
        [
            [
                (
                    x + r * math.cos(a * math.tau / 64),
                    y + r * y_scale * math.sin(a * math.tau / 64),
                    z,
                )
                for a in range(64)
            ]
            for z, r in profile
        ],
        mat,
    )


def smooth_path(knots, steps=8):
    """Interpolating Catmull–Rom spline with clamped end points."""
    result = []
    for i in range(len(knots) - 1):
        a, b, c, d = (
            knots[max(0, i - 1)],
            knots[i],
            knots[i + 1],
            knots[min(i + 2, len(knots) - 1)],
        )
        for j in range(steps):
            t = j / steps
            result.append(
                0.5
                * (
                    (2 * b)
                    + (-a + c) * t
                    + (2 * a - 5 * b + 4 * c - d) * t * t
                    + (-a + 3 * b - 3 * c + d) * t * t * t
                )
            )
    return result + [knots[-1]]


def arm_pose(index, mode, wave):
    path = [Vector(p) for p in ARMS[index]]
    if index == 4 and mode == "watch":
        path = [
            Vector(p)
            for p in [
                (-0.40, -0.06, 0.48),
                (-0.76, -0.08, 0.32),
                (-1.06, -0.13, 0.24),
                (-1.30, 0.02, 0.28),
                (-1.46, -0.02, 0.47),
                (-1.42, -0.13, 0.57),
            ]
        ]
    if index == 4 and mode == "bell":
        path = [
            Vector(p)
            for p in [
                (-0.40, -0.06, 0.48),
                (-0.82, -0.14, 0.40),
                (-1.12, -0.22, 0.53),
                (-1.28, -0.28, 0.87),
                (-1.38, -0.34, 1.16),
                (-1.54, -0.39, 1.25),
                (-1.60, -0.43, 1.14),
            ]
        ]
    if index == 7:
        # Lift from the base, with a smaller counter-curl at the very tip.
        for i, p in enumerate(path[2:], 2):
            t = (i - 1) / (len(path) - 2)
            angle = wave * (1.28 - 0.38 * t)
            x, z = p.x - 0.74, p.z - 0.30
            p.x = 0.74 + x * math.cos(angle) - z * math.sin(angle) - 0.13 * t * t * wave
            p.z = 0.30 + x * math.sin(angle) + z * math.cos(angle) + 0.20 * t * t * wave
            p.y -= 0.16 * t * wave
    return smooth_path(path)


def tube(path, mat, name):
    rings, sections = [], []
    for i, p in enumerate(path):
        t = i / (len(path) - 1)
        tangent = (path[min(i + 1, len(path) - 1)] - path[max(i - 1, 0)]).normalized()
        side = Vector((-tangent.y, tangent.x, 0)).normalized()
        up = tangent.cross(side).normalized()
        radius = 0.009 + 0.357 * (1 - t) ** 0.69
        rings.append(
            [
                p
                + radius
                * (side * math.cos(a * math.tau / 24) + up * 0.84 * math.sin(a * math.tau / 24))
                for a in range(24)
            ]
        )
        sections.append((p, side, up, radius))
    return surface(name, rings, mat), sections


def suckers(sections, mat):
    # Shallow dishes wrap the viewer-facing flank of the raised curls.
    for index in range(len(sections) // 2, len(sections) - 2, 4):
        center, side, up, radius = sections[index]
        tangent = side.cross(up)
        front = Vector((0, -0.9, -0.48))
        facing = (front - tangent * tangent.dot(front)).normalized()
        across = tangent.cross(facing)
        for spread in (-0.32, 0.32):
            normal = (facing + across * spread).normalized()
            size = 0.20 * radius
            profile = [
                (0, size * 0.65),
                (0.12 * size, size),
                (0.34 * size, size * 0.93),
                (0.40 * size, size * 0.70),
                (0.26 * size, size * 0.50),
                (0.06 * size, size * 0.31),
            ]
            disk = radial(profile, 0, 0, mat, "Porcelain sucker")
            disk.location = center + radius * (side * normal.dot(side) + 0.84 * up * normal.dot(up))
            disk.rotation_euler = normal.to_track_quat("Z", "Y").to_euler()


def join_skin(pieces):
    bpy.ops.object.select_all(action="DESELECT")
    for obj in pieces:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = pieces[0]
    bpy.ops.object.join()
    body = bpy.context.object
    body.data.remesh_voxel_size = 0.029
    bpy.ops.object.voxel_remesh()
    smooth = body.modifiers.new("Blend the web into the mantle", "SMOOTH")
    smooth.factor, smooth.iterations = 0.95, 7
    for face in body.data.polygons:
        face.use_smooth = True


def prop(mode, gold, cream, wave):
    if mode == "watch":
        # An open-frame hourglass reads even at the smallest window size.
        x, y = -1.36, -0.56
        glass, shader = material("Sea glass", "9cdedb", 0.15, coat=0.75)
        shader.inputs["Transmission Weight"].default_value = 0.84
        sand, _ = material("Warm sand", "e5ab5f", 0.72)
        radial([(0.03, 0.18), (0.06, 0.22), (0.10, 0.22), (0.13, 0.18)], x, y, gold, "Foot")
        radial([(0.71, 0.19), (0.75, 0.22), (0.79, 0.22), (0.81, 0.18)], x, y, gold, "Crown")
        radial(
            [
                (0.12, 0.12),
                (0.14, 0.16),
                (0.21, 0.176),
                (0.30, 0.15),
                (0.41, 0.046),
                (0.44, 0.042),
                (0.55, 0.14),
                (0.64, 0.174),
                (0.70, 0.16),
                (0.72, 0.12),
            ],
            x,
            y,
            glass,
            "Glass",
        )
        radial(
            [(0.14, 0.12), (0.18, 0.15), (0.22, 0.15), (0.26, 0.11), (0.29, 0.007)],
            x,
            y,
            sand,
            "Lower sand",
        )
        radial([(0.42, 0.012), (0.55, 0.115), (0.60, 0.147)], x, y, sand, "Upper sand")
        for angle in (0.45, 2.64, 4.78):
            radial(
                [(0.10, 0.014), (0.74, 0.014)],
                x + 0.184 * math.cos(angle),
                y + 0.184 * math.sin(angle),
                gold,
                "Stay",
            )
    elif mode == "bell":
        bell = radial(
            [
                (0, 0.245),
                (0.025, 0.257),
                (0.05, 0.246),
                (0.072, 0.214),
                (0.13, 0.19),
                (0.25, 0.155),
                (0.32, 0.113),
                (0.355, 0.05),
                (0.39, 0.045),
                (0.47, 0.031),
                (0.59, 0.042),
                (0.63, 0.062),
                (0.67, 0.045),
            ],
            0,
            0,
            gold,
            "Handbell",
        )
        bell.location = (-1.53, -0.46, 0.63)
        bell.rotation_euler.y = -0.25 - 0.12 * wave
        bead = ellipsoid("Bell clapper", (0, 0, 0), (0.065, 0.065, 0.072), cream)
        bead.parent = bell
        bead.location = (0, 0, -0.023)


def model(mode, wave):
    skin = clay(mode)
    pearl, _ = material(
        "Mother of pearl",
        {"rest": "c5a8ce", "watch": "9fc7cf", "bell": "dfab99"}[mode],
        0.36,
        coat=0.22,
    )
    milk, _ = material("Warm porcelain", "f5eee1", 0.31, coat=0.18)
    ink, _ = material("Obsidian", "1d1925", 0.16, coat=0.28)
    gold, _ = material("Brushed brass", "d5a75f", 0.28, metal=0.73, coat=0.12)
    # The mantle is a turned form with an asymmetric crown and a continuous neck.
    profile = [
        (0.10, 0.32),
        (0.22, 0.57),
        (0.41, 0.68),
        (0.59, 0.64),
        (0.73, 0.575),
        (0.90, 0.62),
        (1.07, 0.73),
        (1.28, 0.81),
        (1.49, 0.83),
        (1.70, 0.79),
        (1.89, 0.67),
        (2.04, 0.49),
        (2.13, 0.28),
        (2.17, 0.06),
    ]
    lean = {"rest": -0.095, "watch": -0.22, "bell": 0.03}[mode] + 0.06 * wave

    def offset(z):
        return lean * max(0, z - 0.48), 0.075 * max(0, z - 0.6)

    rings = []
    for z, radius, _ in smooth_path([Vector((z, r, 0)) for z, r in profile], 4):
        dx, dy = offset(z)
        rings.append(
            [
                (
                    dx + radius * math.cos(a * math.tau / 64),
                    dy + radius * 0.89 * math.sin(a * math.tau / 64),
                    z,
                )
                for a in range(64)
            ]
        )
    pieces = [surface("Mantle", rings, skin)]
    for i in range(8):
        arm, sections = tube(arm_pose(i, mode, wave), skin, f"Arm {i+1}")
        pieces.append(arm)
        if i in (4, 5, 6, 7):
            suckers(sections, pearl)
    join_skin(pieces)
    # Shift both beads over their own sclera when attention turns to the glass.
    eye_z = 1.20 if mode == "watch" else 1.26
    for angle in (-0.56, 0.22):
        dx, dy = offset(eye_z)
        normal = Vector((math.sin(angle), -math.cos(angle), 0.06)).normalized()
        center = Vector((dx + 0.785 * normal.x, dy + 0.695 * normal.y, eye_z))
        eye = ellipsoid("Porcelain eye", center, (0.142, 0.096, 0.173), milk)
        eye.rotation_euler.z = angle
        # A small wink on the upward sweep, with both eyes open in the held pose.
        wink = 0.55 * math.exp(-(((wave - 0.74) / 0.09) ** 2)) if angle > 0 else 0
        eye.scale.z *= 1 - wink
        gaze = {"rest": -0.015, "watch": -0.053, "bell": -0.034}[mode] + 0.026 * wave
        pupil = ellipsoid(
            "Glass pupil",
            center + Vector((gaze, -0.094, -0.035 if mode == "watch" else 0.012)),
            (0.079, 0.040, 0.100),
            ink,
        )
        pupil.rotation_euler.z = angle * 0.4 + gaze * 5
        pupil.scale.z *= 1 - wink
        ellipsoid(
            "Reflected softbox",
            pupil.location + Vector((-0.020, -0.031, 0.049 * (1 - wink))),
            (0.012, 0.006, 0.015),
            milk,
            16,
        )
    prop(mode, gold, milk, wave)


def aim(obj, point):
    obj.rotation_euler = (Vector(point) - obj.location).to_track_quat("-Z", "Y").to_euler()


def studio():
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.samples = 96
    scene.cycles.use_denoising = False
    scene.cycles.transparent_max_bounces = 5
    scene.render.resolution_x, scene.render.resolution_y = WIDTH, HEIGHT
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = True
    scene.render.image_settings.color_mode = "RGBA"
    scene.view_settings.view_transform = "AgX"
    scene.view_settings.look = "AgX - Medium High Contrast"
    scene.world.color = (0.14, 0.14, 0.16)
    bpy.ops.mesh.primitive_plane_add(size=200, location=(0, 0, 0.025))
    floor = bpy.context.object
    floor.visible_camera = False
    floor.data.materials.append(material("Bounce card", "9c939a", 0.9)[0])
    bpy.ops.object.camera_add(location=(0.05, -9.5, 4.30))
    camera = bpy.context.object
    aim(camera, (0.02, -0.15, 0.99))
    camera.data.type, camera.data.ortho_scale = "ORTHO", 4.45
    scene.camera = camera
    for name, xyz, power, size, tint in [
        ("Tall softbox", (-3.8, -4.1, 6), 540, 3.4, (1.0, 0.93, 0.86)),
        ("Fill", (4, -2, 3.2), 135, 4, (0.78, 0.84, 1)),
        ("Edge light", (1.3, 3.1, 4.6), 400, 3, (1, 0.86, 0.72)),
        ("Eyelight", (-1.5, -3, 2.6), 22, 0.7, (1, 0.96, 0.93)),
    ]:
        bpy.ops.object.light_add(type="AREA", location=xyz)
        light = bpy.context.object
        light.name = name
        light.data.energy, light.data.shape, light.data.size, light.data.color = (
            power,
            "DISK",
            size,
            tint,
        )
        aim(light, (0, 0, 0.8))


def with_shadow(pose, mode, wave):
    """Project the contact patch onto the same camera plane as the sculpture."""
    scene = bpy.context.scene

    def pixel(x, y):
        uv = world_to_camera_view(scene, scene.camera, Vector((x, y, 0.025)))
        return uv.x * WIDTH, (1 - uv.y) * HEIGHT

    shadow = Image.new("RGBA", pose.size)
    for blur, opacity, offset in [(10, 41, 0.10), (3.4, 66, 0.026)]:
        mask = Image.new("L", pose.size)
        draw = ImageDraw.Draw(mask)

        def patch(pos, rx, ry, value):
            x, y = pixel(pos.x + offset, pos.y + offset)
            draw.ellipse((x - rx, y - ry, x + rx, y + ry), fill=value)

        patch(Vector((0, 0, 0)), 68, 22, opacity)
        for i in range(8):
            path = arm_pose(i, mode, wave)
            for j, pos in enumerate(path):
                radius = 0.009 + 0.357 * (1 - j / (len(path) - 1)) ** 0.69
                clearance = max(0, pos.z - radius * 0.84)
                alpha = int(opacity * math.exp(-clearance * (2.3 if blur > 5 else 7)))
                patch(pos, radius * 100 + blur, radius * 36 + blur, alpha)
        if mode == "watch":
            patch(Vector((-1.36, -0.56, 0)), 21, 7, opacity)
        layer = Image.new("RGBA", pose.size, (23, 19, 29, 255))
        layer.putalpha(mask.filter(ImageFilter.GaussianBlur(blur)))
        shadow = Image.alpha_composite(shadow, layer)
    return Image.alpha_composite(shadow, pose.convert("RGBA"))


def pack_stills():
    # The timer and reduced-motion mode need only the two end poses. Loading
    # this small shared plate makes starting and dismissing a timer immediate.
    stills = Image.new("RGBA", (WIDTH * len(STATES), HEIGHT * 2))
    for column, mode in enumerate(STATES):
        with Image.open(DEST / f"{mode}.webp") as atlas:
            for row, index in enumerate((0, COLUMNS * ROWS - 1)):
                x, y = index % COLUMNS * WIDTH, index // COLUMNS * HEIGHT
                stills.paste(
                    atlas.crop((x, y, x + WIDTH, y + HEIGHT)), (column * WIDTH, row * HEIGHT)
                )
    stills.save(DEST / "stills.webp", lossless=True, method=6)


def run():
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    DEST.mkdir(exist_ok=True, parents=True)
    with tempfile.TemporaryDirectory() as scratch:
        for mode in ([args[0]] if args else STATES):
            atlas = Image.new("RGBA", (WIDTH * COLUMNS, HEIGHT * ROWS))
            for frame in ([int(args[1])] if args else range(COLUMNS * ROWS)):
                bpy.ops.wm.read_factory_settings(use_empty=True)
                bpy.context.scene.world = bpy.data.worlds.new("Studio")
                studio()
                model(mode, frame / (COLUMNS * ROWS - 1))
                png = Path(scratch) / "pose.png"
                bpy.context.scene.render.filepath = str(png)
                bpy.ops.render.render(write_still=True)
                with Image.open(png) as rendered:
                    im = with_shadow(rendered, mode, frame / (COLUMNS * ROWS - 1))
                    if args:
                        im.save(Path.cwd() / f"{mode}-{frame}.png")
                    else:
                        atlas.paste(im, (frame % COLUMNS * WIDTH, frame // COLUMNS * HEIGHT))
            if not args:
                atlas.save(DEST / f"{mode}.webp", quality=91, method=6)

    if not args:
        pack_stills()


if __name__ == "__main__":
    run()
