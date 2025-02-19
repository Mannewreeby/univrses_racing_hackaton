use std::{
    net::UdpSocket,
    num::NonZeroUsize,
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::{
    app::{App, Startup, Update}, asset::{AssetServer, Handle}, diagnostic::LogDiagnosticsPlugin, math::Vec3, prelude::{
        Camera3dBundle, Commands, Entity, EventReader, EventWriter, IntoSystemConfigs, ParamSet, Query, Res, ResMut, Transform, With
    }, scene::Scene, DefaultPlugins
};
use bevy_garage_camera::CarCameraPlugin;
use bevy_garage_car::{Car, CarRes, CarWheels, Wheel, esp_system, spawn_car};
use bevy_garage_track::{
    SpawnCarOnTrackEvent, TrackConfig, TrackPlugin, spawn_car_on_track, track_start_system,
};
use bevy_rapier3d::{
    plugin::{NoUserData, RapierConfiguration, RapierContext, RapierPhysicsPlugin, TimestepMode},
    render::RapierDebugRenderPlugin,
};
use bevy_renet::{
    RenetServerPlugin,
    renet::{
        RenetServer, ServerEvent,
        transport::{NetcodeServerTransport, ServerAuthentication, ServerConfig},
    },
    transport::NetcodeServerPlugin,
};
use hackaton::{
    ClientChannel, NetworkedEntities, Player, PlayerInput, SERVER_PROTOCOL_ID, ServerChannel,
    ServerLobby, ServerMessages, connection_config, shared_systems::setup_level,
};

pub fn start_server() -> (RenetServer, NetcodeServerTransport) {
    let server = RenetServer::new(connection_config());

    let addr = match std::env::var("APP_SERVER") {
        Ok(addr) => addr,
        _ => "127.0.0.1:5000".to_string(),
    };

    println!("Starting server on {addr}");

    let public_addr = addr.parse().expect("Could not parse server addr");
    let socket = UdpSocket::bind(public_addr).expect("Could not bind udp socket addr");

    let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

    let server_configuration = ServerConfig {
        current_time,
        max_clients: 12,
        protocol_id: SERVER_PROTOCOL_ID,
        public_addresses: vec![public_addr],
        authentication: ServerAuthentication::Unsecure,
    };

    let transport = NetcodeServerTransport::new(server_configuration, socket)
        .expect("Could not setup transport server");

    (server, transport)
}

pub fn main() {
    let mut app = App::new();
    app.insert_resource(bevy_garage_car::CarRes {
        show_rays: true,
        car_scene: None,
        wheel_scene: None,
    });

    app.add_plugins((
        DefaultPlugins,
        RapierDebugRenderPlugin::default(),
        bevy_egui::EguiPlugin,
    ));

    app.add_plugins((
        RenetServerPlugin,
        NetcodeServerPlugin,
        RapierPhysicsPlugin::<NoUserData>::default(),
        LogDiagnosticsPlugin::default(),
        TrackPlugin,
        CarCameraPlugin,
    ));

    app.insert_resource(RapierConfiguration {
        timestep_mode: TimestepMode::Variable {
            max_dt: 1. / 60.,
            time_scale: 1.,
            substeps: 10,
        },
        gravity: Vec3::new(0., -9.8, 0.),
        physics_pipeline_active: true,
        query_pipeline_active: true,
        scaled_shape_subdivision: 3,
        force_update_from_transform_changes: true,
    });

    app.insert_resource(ServerLobby::default());
    app.add_event::<SpawnCarOnTrackEvent>();

    let (server, transport) = start_server();
    app.insert_resource(server).insert_resource(transport);

    app.insert_resource(renet_visualizer::RenetServerVisualizer::<200>::default());

    app.add_systems(
        Update,
        (
            move_players_system,
            esp_system.after(move_players_system),
            server_update_system,
            server_network_sync,
            spawn_car_system,
            update_visulizer_system,
        ),
    );

    app.add_systems(
        Startup,
        (
            setup_level,
            rapier_config_start_system,
            car_start_system,
            track_start_system,
        ),
    );

    app.run();
}

fn update_visulizer_system(
    mut egui_contexts: bevy_egui::EguiContexts,
    mut visualizer: ResMut<renet_visualizer::RenetServerVisualizer<200>>,
    server: Res<RenetServer>,
) {
    visualizer.update(&server);
    visualizer.show_window(egui_contexts.ctx_mut());
}

pub fn car_start_system(mut config: ResMut<CarRes>, asset_server: Res<AssetServer>) {
    let wheel_gl: Handle<Scene> = asset_server.load("wheelRacing.glb#Scene0");
    config.wheel_scene = Some(wheel_gl.clone());
    let car_gl: Handle<Scene> = asset_server.load("car-race.glb#Scene0");
    config.car_scene = Some(car_gl.clone());
}

fn rapier_config_start_system(mut c: ResMut<RapierContext>) {
    c.integration_parameters.num_solver_iterations = NonZeroUsize::new(6).unwrap();
    c.integration_parameters.warmstart_coefficient = 0.;
    c.integration_parameters.contact_natural_frequency = 50.;
    c.integration_parameters.contact_damping_ratio = 50.;
    // c.integration_parameters.num_internal_pgs_iterations = 16;
    // c.integration_parameters.num_additional_friction_iterations = 8;
    dbg!(c.integration_parameters);
}
fn server_update_system(
    mut server_events: EventReader<ServerEvent>,
    mut cmd: Commands,
    mut lobby: ResMut<ServerLobby>,
    mut server: ResMut<RenetServer>,
    players: Query<(Entity, &Player, &Transform)>,
    #[cfg(feature = "graphics")] car_res: Res<bevy_garage_car::CarRes>,
    #[cfg(feature = "graphics")] mut visualizer: ResMut<
        renet_visualizer::RenetServerVisualizer<200>,
    >,
    track_config: ResMut<TrackConfig>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                println!("Player {} connected.", client_id);
                #[cfg(feature = "graphics")]
                visualizer.add_client(*client_id);

                for (entity, player, transform) in players.iter() {
                    let translation: [f32; 3] = transform.translation.into();
                    let message = bincode::serialize(&ServerMessages::PlayerCreate {
                        id: player.id,
                        entity,
                        position: translation,
                    })
                    .unwrap();
                    server.send_message(*client_id, ServerChannel::ServerMessages, message);
                }
                let (translation, quat) = track_config.get_transform_by_meter(0.);
                let transform = Transform::from_translation(translation).with_rotation(quat);
                let player_entity = spawn_car(
                    &mut cmd,
                    car_res.car_scene.as_ref().unwrap(),
                    car_res.wheel_scene.as_ref().unwrap(),
                    false,
                    transform,
                );
                cmd.entity(player_entity)
                    .insert(Player {
                        id: client_id.raw(),
                    })
                    .insert(PlayerInput::default());

                lobby.players.insert(client_id.raw(), player_entity);

                let translation: [f32; 3] = transform.translation.into();
                let message = bincode::serialize(&ServerMessages::PlayerCreate {
                    id: client_id.raw(),
                    entity: player_entity,
                    position: translation,
                })
                .unwrap();
                server.broadcast_message(ServerChannel::ServerMessages, message);
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                println!("Player {} disconnected: {}", client_id, reason);
                #[cfg(feature = "graphics")]
                visualizer.remove_client(*client_id);
                if let Some(player_entity) = lobby.players.remove(&client_id.raw()) {
                    cmd.entity(player_entity).despawn();
                }

                let message = bincode::serialize(&ServerMessages::PlayerRemove {
                    id: client_id.raw(),
                })
                .unwrap();
                server.broadcast_message(ServerChannel::ServerMessages, message);
            }
        }
    }

    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, ClientChannel::Input) {
            let input: PlayerInput = bincode::deserialize(&message).unwrap();
            if let Some(player_entity) = lobby.players.get(&client_id.raw()) {
                cmd.entity(*player_entity).insert(input);
            }
        }
    }
}
pub fn setup_simple_camera(mut commands: Commands) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-20.5, 30.0, 20.5).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}
fn server_network_sync(
    mut server: ResMut<RenetServer>,
    mut tr_set: ParamSet<(
        Query<(Entity, &Transform, &CarWheels), With<Player>>,
        Query<&Transform, With<Wheel>>,
    )>,
) {
    let mut networked_entities = NetworkedEntities::default();
    let mut wheels_all: Vec<[Entity; 4]> = vec![];
    for (entity, transform, wheels) in tr_set.p0().iter() {
        networked_entities.entities.push(entity);
        networked_entities
            .positions
            .push(transform.translation.into());
        networked_entities
            .orientations
            .push(transform.rotation.into());

        wheels_all.push(wheels.entities);
    }

    for wheels in wheels_all {
        networked_entities.wheel_positions.push([
            tr_set.p1().get(wheels[0]).unwrap().translation.into(),
            tr_set.p1().get(wheels[1]).unwrap().translation.into(),
            tr_set.p1().get(wheels[2]).unwrap().translation.into(),
            tr_set.p1().get(wheels[3]).unwrap().translation.into(),
        ]);
        networked_entities.wheel_orientations.push([
            tr_set.p1().get(wheels[0]).unwrap().rotation.into(),
            tr_set.p1().get(wheels[1]).unwrap().rotation.into(),
            tr_set.p1().get(wheels[2]).unwrap().rotation.into(),
            tr_set.p1().get(wheels[3]).unwrap().rotation.into(),
        ]);
    }

    let sync_message = bincode::serialize(&networked_entities).unwrap();
    server.broadcast_message(ServerChannel::NetworkedEntities, sync_message);
}

pub fn spawn_car_start_system(mut car_spawn_events: EventWriter<SpawnCarOnTrackEvent>) {
    car_spawn_events.send(SpawnCarOnTrackEvent {
        player: true,
        index: 0,
        position: Some(0.),
    });
}

pub fn spawn_car_system(
    mut events: EventReader<SpawnCarOnTrackEvent>,
    mut cmd: Commands,
    track_config: ResMut<TrackConfig>,
    car_res: ResMut<CarRes>,
) {
    for spawn_event in events.read() {
        dbg!(spawn_event);

        let (transform, init_meters) = if let Some(init_meters) = spawn_event.position {
            let (translate, quat) = track_config.get_transform_by_meter(init_meters);
            let transform = Transform::from_translation(translate).with_rotation(quat);
            (transform, init_meters)
        } else {
            track_config.get_transform_random()
        };

        spawn_car_on_track(
            &mut cmd,
            car_res.car_scene.as_ref().unwrap(),
            car_res.wheel_scene.as_ref().unwrap(),
            spawn_event.player,
            transform,
            spawn_event.index,
            init_meters,
        );
    }
}

fn move_players_system(mut query: Query<(&PlayerInput, &mut Car)>) {
    for (input, mut car) in query.iter_mut() {
        if input.forward {
            car.gas = 1.;
        } else {
            car.gas = 0.;
        }
        if input.brake {
            car.brake = 1.;
        } else {
            car.brake = 0.;
        }
        if input.left {
            car.steering = -1.;
        }
        if input.right {
            car.steering = 1.;
        }
        if !input.left && !input.right {
            car.steering = 0.;
        }
    }
}
