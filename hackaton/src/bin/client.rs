use std::{
    net::UdpSocket,
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::{
    DefaultPlugins,
    app::{App, Startup, Update},
    diagnostic::FrameTimeDiagnosticsPlugin,
    input::ButtonInput,
    math::Quat,
    prelude::{
        Commands, Component, Entity, IntoSystemConfigs, KeyCode, Local, Query, Res, ResMut,
        Resource, Transform, With,
    },
    utils::HashMap,
};
use bevy_egui::{EguiContexts, EguiPlugin};
use bevy_garage_camera::CarCameraPlugin;
use bevy_garage_car::{CarWheels, Wheel};
use bevy_garage_track::{TrackPlugin, track_start_system};
use bevy_renet::{
    RenetClientPlugin, client_connected,
    renet::{
        RenetClient,
        transport::{ClientAuthentication, NetcodeClientTransport},
    },
    transport::{self, NetcodeClientPlugin},
};
use hackaton::{
    ClientChannel, NetworkedEntities, PlayerInput, SERVER_PROTOCOL_ID, ServerChannel,
    ServerMessages, connection_config, shared_systems::setup_level,
};
use renet_visualizer::{RenetClientVisualizer, RenetVisualizerStyle};

#[derive(Component)]
struct ControlledPlayer;

#[derive(Default, Resource)]
struct NetworkMapping(HashMap<Entity, Entity>);

#[derive(Debug)]
struct PlayerInfo {
    client_entity: Entity,
    server_entity: Entity,
}

#[derive(Debug, Default, Resource)]
struct ClientLobby {
    players: HashMap<u64, PlayerInfo>,
}

fn start_renet_client() -> (RenetClient, NetcodeClientTransport) {
    let client = RenetClient::new(connection_config());
    let addr = match std::env::var("APP_SERVER") {
        Ok(addr) => addr,
        _ => "127.0.0.1:5000".to_string(),
    };

    let server_addr = addr.parse().unwrap();
    let socket = UdpSocket::bind("127.0.0.1:0").expect("Could not bind socket addr");
    let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let client_id = current_time.as_millis() as u64;
    let authentication = ClientAuthentication::Unsecure {
        protocol_id: SERVER_PROTOCOL_ID,
        client_id,
        server_addr,
        user_data: None,
    };

    let transport = NetcodeClientTransport::new(current_time, authentication, socket)
        .expect("Could not create client netcode transport");

    return (client, transport);
}

pub fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins,
        RenetClientPlugin,
        NetcodeClientPlugin,
        FrameTimeDiagnosticsPlugin,
        EguiPlugin,
        CarCameraPlugin,
        TrackPlugin,
    ));

    app.insert_resource(bevy_garage_car::CarRes {
        show_rays: true,
        ..Default::default()
    });
    app.insert_resource(RenetClientVisualizer::<200>::new(
        RenetVisualizerStyle::default(),
    ));
    app.insert_resource(ClientLobby::default());
    app.insert_resource(NetworkMapping::default());

    let (client, transport) = start_renet_client();
    app.insert_resource(client);
    app.insert_resource(transport);
    app.add_systems(
        Startup,
        (
            setup_level,
            bevy_garage_car::car_start_system,
            track_start_system,
        ),
    );

    app.insert_resource(PlayerInput::default());
    app.add_systems(Update, update_visulizer_system);

    app.add_systems(
        Update,
        ((client_sync_players, client_send_input, player_input).run_if(client_connected),),
    );

    app.run();
}

fn update_visulizer_system(
    mut egui_contexts: EguiContexts,
    mut visualizer: ResMut<RenetClientVisualizer<200>>,
    client: Res<RenetClient>,
    mut show_visualizer: Local<bool>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    visualizer.add_network_info(client.network_info());
    if keyboard_input.just_pressed(KeyCode::F1) {
        *show_visualizer = !*show_visualizer;
    }
    if *show_visualizer {
        visualizer.show_window(egui_contexts.ctx_mut());
    }
}

fn player_input(keyboard_input: Res<ButtonInput<KeyCode>>, mut player_input: ResMut<PlayerInput>) {
    player_input.left = keyboard_input.pressed(KeyCode::ArrowLeft);
    player_input.right = keyboard_input.pressed(KeyCode::ArrowRight);
    player_input.forward = keyboard_input.pressed(KeyCode::ArrowUp);
    player_input.brake = keyboard_input.pressed(KeyCode::ArrowDown);
}

fn client_send_input(player_input: Res<PlayerInput>, mut client: ResMut<RenetClient>) {
    let input_message = bincode::serialize(&*player_input).unwrap();
    client.send_message(ClientChannel::Input, input_message);
}

fn client_sync_players(
    mut cmd: Commands,
    mut client: ResMut<RenetClient>,
    transport: Res<NetcodeClientTransport>,
    mut lobby: ResMut<ClientLobby>,
    mut network_mapping: ResMut<NetworkMapping>,
    car_res: Res<bevy_garage_car::CarRes>,
    car_wheels: Query<&CarWheels>,
    mut wheel_query: Query<&mut Transform, With<Wheel>>,
) {
    let client_id = transport.client_id();
    while let Some(message) = client.receive_message(ServerChannel::ServerMessages) {
        let server_message = bincode::deserialize(&message).unwrap();
        match server_message {
            ServerMessages::PlayerCreate {
                id,
                position,
                entity,
            } => {
                println!("Player {} connected.", id);

                let is_player = client_id.raw() == id;

                let transform: Transform =
                    Transform::from_xyz(position[0], position[1], position[2]);
                let client_entity = bevy_garage_car::spawn_car(
                    &mut cmd,
                    &car_res.car_scene.as_ref().unwrap(),
                    &car_res.wheel_scene.as_ref().unwrap(),
                    is_player,
                    transform,
                );

                if is_player {
                    cmd.entity(client_entity).insert(ControlledPlayer);
                }

                let player_info = PlayerInfo {
                    server_entity: entity,
                    client_entity,
                };
                lobby.players.insert(id, player_info);
                network_mapping.0.insert(entity, client_entity);
            }
            ServerMessages::PlayerRemove { id } => {
                println!("Player {} disconnected.", id);
                if let Some(PlayerInfo {
                    server_entity,
                    client_entity,
                }) = lobby.players.remove(&id)
                {
                    cmd.entity(client_entity).despawn();
                    network_mapping.0.remove(&server_entity);
                }
            }
        }
    }

    while let Some(message) = client.receive_message(ServerChannel::NetworkedEntities) {
        let networked_entities: NetworkedEntities = bincode::deserialize(&message).unwrap();

        for i in 0..networked_entities.entities.len() {
            if let Some(entity) = network_mapping.0.get(&networked_entities.entities[i]) {
                let translation = networked_entities.positions[i].into();
                let rotation: Quat = Quat::from_array(networked_entities.orientations[i]);
                let transform = Transform {
                    translation,
                    rotation,
                    ..Default::default()
                };
                cmd.entity(*entity).insert(transform);

                let translations = networked_entities.wheel_positions[i];
                let rotations = networked_entities.wheen_orientations[i];

                let car_wheels = car_wheels.get(*entity);
                if let Ok(car_wheels) = car_wheels {
                    for (i, e) in car_wheels.entities.iter().enumerate() {
                        let mut wheel_transform = wheel_query.get_mut(*e).unwrap();
                        wheel_transform.translation = translations[i].into();
                        wheel_transform.rotation = Quat::from_array(rotations[i]);
                    }
                }
            }
        }
    }
}
