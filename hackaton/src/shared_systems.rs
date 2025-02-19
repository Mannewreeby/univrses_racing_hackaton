use std::f32::consts::PI;

use bevy::{
    asset::Assets,
    color::Color,
    math::{Quat, Vec3},
    pbr::{DirectionalLight, DirectionalLightBundle, PbrBundle, StandardMaterial},
    prelude::{Commands, Cuboid, Mesh, ResMut, Transform, TransformBundle},
};
use bevy_garage_car::STATIC_GROUP;
use bevy_rapier3d::prelude::{
    Collider, ColliderScale, CollisionGroups, Friction, Group, Restitution, RigidBody,
};

pub fn setup_level(
    mut cmd: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let size = 1000.;
    let cuboid = Collider::cuboid(size / 2., 0.5, size / 2.);
    let transform = Transform::from_xyz(0.0, -1.0, 0.0);

    let mut cuboid_cmd = cmd.spawn((
        cuboid,
        RigidBody::Fixed,
        ColliderScale::Absolute(Vec3::ONE),
        CollisionGroups::new(STATIC_GROUP, Group::ALL),
        Friction::coefficient(3.),
        Restitution::coefficient(0.),
    ));
    cuboid_cmd.insert(PbrBundle {
        mesh: meshes.add(Mesh::from(Cuboid::new(size, 1., size))),
        material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
        transform,
        ..Default::default()
    });
    cuboid_cmd.insert(TransformBundle::from_transform(transform));

    cmd.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..Default::default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..Default::default()
        },
        ..Default::default()
    });
}
