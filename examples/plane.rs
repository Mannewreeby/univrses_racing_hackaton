use bevy::prelude::*;
use bevy_garage_car::{car_start_system, esp_system, spawn_car, Car, CarRes};
use bevy_rapier3d::prelude::*;

fn main() {
    let mut rapier_config = RapierConfiguration::new(1.);
    rapier_config.timestep_mode = TimestepMode::Variable {
        max_dt: 1. / 60.,
        time_scale: 1.,
        substeps: 5,
    };
    App::new()
        .insert_resource(rapier_config)
        .add_plugins((
            DefaultPlugins,
            RapierPhysicsPlugin::<NoUserData>::default(),
            RapierDebugRenderPlugin::default(),
        ))
        .insert_resource(CarRes {
            show_rays: true,
            ..default()
        })
        .add_systems(
            Startup,
            (
                rapier_config_start_system,
                plane_start,
                car_start_system,
                spawn_car_system.after(car_start_system),
            ),
        )
        .add_systems(Update, (input_system, esp_system.after(input_system)))
        .run();
}

fn rapier_config_start_system(mut c: ResMut<RapierContext>) {
    // c.integration_parameters.max_velocity_iterations = 64;
    // c.integration_parameters.max_velocity_friction_iterations = 64;
    // c.integration_parameters.max_stabilization_iterations = 16;
    // c.integration_parameters.erp = 0.99;
}

fn spawn_car_system(mut cmd: Commands, car_res: Res<CarRes>) {
    spawn_car(
        &mut cmd,
        &car_res.car_scene.as_ref().unwrap(),
        &car_res.wheel_scene.as_ref().unwrap(),
        true,
        Transform::from_translation(Vec3 {
            x: 0.,
            y: 1.,
            z: 0.,
        }),
    );
}

fn plane_start(
    mut cmd: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let size = 100.;
    let (cols, rows) = (10, 10);

    cmd.spawn((
        PbrBundle {
            mesh: meshes.add(Plane3d::default().mesh().size(size, size)),
            material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
            ..default()
        },
        RigidBody::Fixed,
        ColliderScale::Absolute(Vec3::ONE),
        Friction::coefficient(3.),
        Restitution::coefficient(0.),
        Collider::heightfield(vec![0.; rows * cols], rows, cols, Vec3::new(size, 0., size)),
    ));

    cmd.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    cmd.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0., 10., 20.).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn input_system(input: Res<ButtonInput<KeyCode>>, mut cars: Query<&mut Car>) {
    for mut car in cars.iter_mut() {
        if input.pressed(KeyCode::ArrowUp) {
            car.gas = 1.;
        }
        if input.just_released(KeyCode::ArrowUp) {
            car.gas = 0.;
        }

        if input.pressed(KeyCode::ArrowDown) {
            car.brake = 1.;
        }
        if input.just_released(KeyCode::ArrowDown) {
            car.brake = 0.;
        }

        if input.pressed(KeyCode::ArrowLeft) {
            car.steering = -1.;
        }
        if input.pressed(KeyCode::ArrowRight) {
            car.steering = 1.;
        }
        if input.just_released(KeyCode::ArrowLeft) {
            car.steering = 0.;
        }
        if input.just_released(KeyCode::ArrowRight) {
            car.steering = 0.;
        }
    }
}
