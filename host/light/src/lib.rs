use bevy::{pbr::NotShadowCaster, prelude::*};

pub fn light_start_system(
    mut cmd: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    cmd.insert_resource(AmbientLight {
        color: Color::srgb_u8(210, 220, 240),
        brightness: 80.,
    });

    cmd.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10_000.,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0., 0., 0.),
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_8),
            ..default()
        },
        ..default()
    });

    cmd.spawn((
        PbrBundle {
            // mesh: meshes.add(Mesh::from(shape::Box::default())),
            mesh: meshes.add(Mesh::from(Cuboid::default())),
            material: materials.add(StandardMaterial {
                base_color: Srgba::hex("888888").unwrap().into(),
                unlit: true,
                cull_mode: None,
                ..default()
            }),
            transform: Transform::from_scale(Vec3::splat(10000.0)),
            ..default()
        },
        NotShadowCaster,
    ));
}

const K: f32 = 2.;

pub fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.pressed(KeyCode::KeyH) {
        for mut transform in &mut query {
            transform.rotate_y(time.delta_seconds() * K);
        }
    }
    if input.pressed(KeyCode::KeyL) {
        for mut transform in &mut query {
            transform.rotate_y(-time.delta_seconds() * K);
        }
    }
    if input.pressed(KeyCode::KeyJ) {
        for mut transform in &mut query {
            transform.rotate_x(time.delta_seconds() * K);
        }
    }
    if input.pressed(KeyCode::KeyK) {
        for mut transform in &mut query {
            transform.rotate_x(-time.delta_seconds() * K);
        }
    }
}
