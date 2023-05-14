#![feature(slice_flatten)]
pub mod camera;
mod config;
mod dash;
mod dsp;
mod esp;
pub mod font;
mod input;
pub mod joystick;
mod light;
mod mesh;
mod progress;
mod spawn;
mod track;
use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, pbr::DirectionalLightShadowMap, prelude::*};
use bevy_garage_car::{car::car_start_system, config::CarConfig, spawn::SpawnCarEvent, CarSet};
use bevy_garage_dqn::BrainPlugin;
use bevy_rapier3d::prelude::*;
use config::*;
use dash::*;
use dsp::*;
use esp::*;
use font::*;
use input::*;
use light::*;
use progress::*;
use spawn::*;
use track::*;

#[derive(Resource, Copy, Clone, Debug)]
pub struct PhysicsParams {
    pub max_velocity_iters: usize,
    pub max_velocity_friction_iters: usize,
    pub max_stabilization_iters: usize,
    pub substeps: usize,
}

impl Default for PhysicsParams {
    fn default() -> Self {
        Self {
            max_velocity_iters: 32,
            max_velocity_friction_iters: 32,
            max_stabilization_iters: 8,
            substeps: 10,
        }
    }
}

fn rapier_config_start_system(mut c: ResMut<RapierContext>, ph: Res<PhysicsParams>) {
    c.integration_parameters.max_velocity_iterations = ph.max_velocity_iters;
    c.integration_parameters.max_velocity_friction_iterations = ph.max_velocity_friction_iters;
    c.integration_parameters.max_stabilization_iterations = ph.max_stabilization_iters;
    // c.integration_parameters.max_ccd_substeps = 16;
    // c.integration_parameters.allowed_linear_error = 0.000001;
    c.integration_parameters.erp = 0.99;
    // c.integration_parameters.erp = 1.;
    // c.integration_parameters.max_penetration_correction = 0.0001;
    // c.integration_parameters.prediction_distance = 0.01;
    dbg!(c.integration_parameters);
}

pub fn car_app(app: &mut App, physics_params: PhysicsParams) -> &mut App {
    #[cfg(feature = "brain")]
    let esp_run_after: CarSet = CarSet::Brain;
    #[cfg(not(feature = "brain"))]
    let esp_run_after: CarSet = CarSet::Input;

    app.init_resource::<FontHandle>()
        .insert_resource(physics_params.clone())
        .insert_resource(RapierConfiguration {
            timestep_mode: TimestepMode::Variable {
                max_dt: 1. / 60.,
                time_scale: 1.,
                substeps: physics_params.substeps,
            },
            ..default()
        })
        .insert_resource(Msaa::Sample4)
        .insert_resource(Config::default())
        .insert_resource(CarConfig::default())
        .insert_resource(DirectionalLightShadowMap::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(bevy_fundsp::DspPlugin::default())
        .add_plugin(TrackPlugin)
        .add_plugin(EngineSoundPlugin)
        .add_event::<SpawnCarEvent>()
        .add_startup_systems((
            track_polyline_start_system,
            car_start_system.after(track_polyline_start_system),
            light_start_system,
            dash_start_system,
            rapier_config_start_system,
        ))
        .add_systems((
            spawn_car_system,
            aero_system.in_set(CarSet::Input),
            input_system.in_set(CarSet::Input),
            progress_system.in_set(CarSet::Input),
            esp_system.in_set(CarSet::Esp).after(esp_run_after),
            animate_light_direction,
            dash_fps_system,
            dash_speed_update_system,
        ));

    #[cfg(feature = "brain")]
    {
        app.add_plugin(BrainPlugin);
    }

    #[cfg(feature = "debug_lines")]
    {
        use bevy_prototype_debug_lines::DebugLinesPlugin;
        app.add_plugin(DebugLinesPlugin::with_depth_test(true))
            .add_plugin(RapierDebugRenderPlugin {
                enabled: false,
                style: DebugRenderStyle {
                    rigid_body_axes_length: 0.5,
                    // subdivisions: 50,
                    ..default()
                },
                // | DebugRenderMode::COLLIDER_AABBS
                mode: DebugRenderMode::COLLIDER_SHAPES
                    | DebugRenderMode::RIGID_BODY_AXES
                    | DebugRenderMode::JOINTS
                    | DebugRenderMode::CONTACTS
                    | DebugRenderMode::SOLVER_CONTACTS,
                ..default()
            });
    }
    app
}

// fn display_events_system(
//     mut e_collision: EventReader<CollisionEvent>,
//     mut e_force: EventReader<ContactForceEvent>,
// ) {
//     for collision_e in e_collision.iter() {
//         println!("main collision event: {:?}", collision_e);
//     }

//     for force_e in e_force.iter() {
//         println!(
//             "glomainbal force event: {:?} {:?}",
//             force_e.total_force, force_e.total_force_magnitude
//         );
//     }
// }
