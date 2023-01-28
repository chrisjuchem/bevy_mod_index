use bevy::asset::Asset;
use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy::sprite::{Material2d, MaterialMesh2dBundle, Mesh2dHandle};
use bevy::utils::HashMap;
use bevy_mod_index::prelude::*;
use rand::rngs::ThreadRng;
use rand::{random, seq::IteratorRandom, thread_rng, Rng};
use std::f32::consts::PI;

const N_BALLS: usize = 1000;
const MAX_WIDTH: f32 = 640.;
const MAX_HEIGHT: f32 = 360.;

#[derive(Component)]
struct Velocity(Vec2);
#[derive(Component)]
struct Size(f32);

#[derive(Resource, Default)]
struct Colors(Vec<Handle<ColorMaterial>>);
impl Colors {
    fn random(&self, rng: &mut ThreadRng) -> Handle<ColorMaterial> {
        self.0.iter().choose(rng).unwrap().clone()
    }
}

struct RegionIndex;
impl IndexInfo for RegionIndex {
    type Component = Transform;
    type Value = Region;

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
    mut colors: ResMut<Colors>,
) {
    commands.spawn(Camera2dBundle::default());

    for color in [
        Color::AQUAMARINE,
        Color::BISQUE,
        Color::BLUE,
        Color::CRIMSON,
        Color::DARK_GRAY,
        Color::DARK_GREEN,
        Color::FUCHSIA,
        Color::GOLD,
        Color::INDIGO,
        Color::PINK,
        Color::SALMON,
        Color::PURPLE,
        Color::OLIVE,
        Color::MAROON,
        Color::GREEN,
        Color::SILVER,
        Color::VIOLET,
        Color::WHITE,
        Color::YELLOW_GREEN,
        Color::ORANGE,
    ] {
        colors.0.push(materials.add(ColorMaterial::from(color)));
    }

    let size_range = 2..8;
    let mut mesh_map = HashMap::new();
    for x in size_range.clone() {
        mesh_map.insert(x, meshes.add(shape::Circle::new(x as f32).into()));
    }

    let mut rng = thread_rng();
    for z in 0..N_BALLS {
        let size = rng.gen_range(size_range.clone());

        commands.spawn((
            MaterialMesh2dBundle {
                mesh: mesh_map.get(&size).unwrap().clone().into(),
                material: colors.random(&mut rng),
                transform: Transform::from_xyz(
                    (rng.gen::<f32>() - 0.5) * MAX_WIDTH,
                    (rng.gen::<f32>() - 0.5) * MAX_HEIGHT,
                    z as f32,
                ),
                ..default()
            },
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
    click: Res<Input<MouseButton>>,
    windows: Query<&Window>,
    mut commands: Commands,
) {
    if click.just_pressed(MouseButton::Left) {
        if let Some(mut pos) = windows.single().cursor_position() {
            pos.x -= MAX_WIDTH;
            pos.y -= MAX_HEIGHT;
            let cursor_region = get_region(&pos);

            let mat = colors.random(&mut thread_rng());
            for e in &index.lookup(&cursor_region) {
                commands.entity(*e).insert(mat.clone());
            }
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Colors::default())
        .add_startup_system(setup)
        .add_system(move_balls)
        .add_system(bounce)
        .add_system(update_colors)
        .run();
}
