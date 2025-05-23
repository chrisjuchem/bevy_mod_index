use std::f32::consts::PI;

use bevy::{
    color::palettes,
    math::Vec3Swizzles,
    platform::collections::hash_map::HashMap,
    prelude::*,
};
use bevy_mod_index::prelude::*;
use rand::{rngs::ThreadRng, seq::IteratorRandom, thread_rng, Rng};

const N_BALLS: usize = 1000;
const MAX_WIDTH: f32 = 640.;
const MAX_HEIGHT: f32 = 360.;

#[derive(Component)]
struct Velocity(Vec2);
#[derive(Component)]
struct Size(f32);

#[derive(Resource, Default)]
struct Colors(Vec<MeshMaterial2d<ColorMaterial>>);
impl Colors {
    fn random(&self, rng: &mut ThreadRng) -> MeshMaterial2d<ColorMaterial> {
        self.0.iter().choose(rng).unwrap().clone()
    }
}

struct RegionIndex;
impl IndexInfo for RegionIndex {
    type Component = Transform;
    type Value = Region;
    type Storage = HashmapStorage<Self>;
    const REFRESH_POLICY: IndexRefreshPolicy = IndexRefreshPolicy::WhenRun;

    fn value(t: &Transform) -> Region {
        get_region(&t.translation.xy())
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
enum Region {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    CenterCenter,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

fn get_region(v: &Vec2) -> Region {
    match (v.x / MAX_WIDTH, v.y / MAX_HEIGHT) {
        (x, y) if x < -0.33 && y < -0.33 => Region::BottomLeft,
        (x, y) if x < -0.33 && y > 0.33 => Region::TopLeft,
        (x, _) if x < -0.33 => Region::CenterLeft,
        (x, y) if x > 0.33 && y < -0.33 => Region::BottomRight,
        (x, y) if x > 0.33 && y > 0.33 => Region::TopRight,
        (x, _) if x > 0.33 => Region::CenterRight,
        (_, y) if y < -0.33 => Region::BottomCenter,
        (_, y) if y > 0.33 => Region::TopCenter,
        (_, _) => Region::CenterCenter,
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut color_materials: ResMut<Colors>,
) {
    commands.spawn(Camera2d);

    let colors: [Color; 20] = [
        palettes::css::AQUAMARINE.into(),
        palettes::css::BISQUE.into(),
        palettes::css::BLUE.into(),
        palettes::css::CRIMSON.into(),
        palettes::css::DARK_GRAY.into(),
        palettes::css::DARK_GREEN.into(),
        palettes::css::FUCHSIA.into(),
        palettes::css::GOLD.into(),
        palettes::css::INDIGO.into(),
        palettes::css::PINK.into(),
        palettes::css::SALMON.into(),
        palettes::css::PURPLE.into(),
        palettes::css::OLIVE.into(),
        palettes::css::MAROON.into(),
        palettes::css::GREEN.into(),
        palettes::css::SILVER.into(),
        palettes::css::VIOLET.into(),
        palettes::css::WHITE.into(),
        palettes::css::YELLOW_GREEN.into(),
        palettes::css::ORANGE.into(),
    ];
    for color in colors {
        color_materials
            .0
            .push(MeshMaterial2d(materials.add(ColorMaterial::from(color))));
    }

    let size_range = 2..8;
    let mut mesh_map = HashMap::<_, Mesh2d>::default();
    for x in size_range.clone() {
        mesh_map.insert(x, Mesh2d(meshes.add(Circle::new(x as f32))));
    }

    let mut rng = thread_rng();
    for z in 0..N_BALLS {
        let size = rng.gen_range(size_range.clone());

        commands.spawn((
            mesh_map.get(&size).unwrap().clone(),
            color_materials.random(&mut rng),
            Transform::from_xyz(
                (rng.gen::<f32>() - 0.5) * MAX_WIDTH,
                (rng.gen::<f32>() - 0.5) * MAX_HEIGHT,
                z as f32,
            ),
            Velocity(Vec2::from_angle(rng.gen::<f32>() * 2. * PI) * (rng.gen::<f32>() * 3. + 0.5)),
            Size(size as f32),
        ));
    }
}

fn move_balls(mut balls: Query<(&mut Transform, &Velocity)>) {
    for (mut t, v) in &mut balls {
        t.translation += Vec3::from((v.0, 0.));
    }
}

fn bounce(mut balls: Query<(&Transform, &mut Velocity, &Size)>) {
    for (t, mut v, s) in &mut balls {
        if t.translation.x - s.0 < -MAX_WIDTH || t.translation.x + s.0 > MAX_WIDTH {
            v.0.x *= -1.;
        }
        if t.translation.y - s.0 < -MAX_HEIGHT || t.translation.y + s.0 > MAX_HEIGHT {
            v.0.y *= -1.;
        }
    }
}

fn update_colors(
    mut index: Index<RegionIndex>,
    colors: Res<Colors>,
    click: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    mut commands: Commands,
) {
    if click.just_pressed(MouseButton::Left) {
        if let Some(mut pos) = window.cursor_position() {
            pos.x -= MAX_WIDTH;
            pos.y -= MAX_HEIGHT;
            // convert screen space to world space
            pos.y = -pos.y;
            let cursor_region = get_region(&pos);

            let mat = colors.random(&mut thread_rng());
            for e in index.lookup(&cursor_region) {
                commands.entity(e).insert(mat.clone());
            }
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Colors::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (move_balls, bounce))
        .add_systems(Update, update_colors)
        .run();
}
