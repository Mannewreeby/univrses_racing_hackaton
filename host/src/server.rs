use std::{net::UdpSocket, time::Duration};

use bevy::{
    input::ButtonInput,
    log::tracing_subscriber::fmt::time::SystemTime,
    math::Vec3,
    prelude::{
        Commands, Component, Entity, EventReader, KeyCode, Local, Query, Res, ResMut, Resource,
        Transform,
    },
    utils::HashMap,
};
use bevy_egui::EguiContext;
use bevy_garage_car::spawn_car;
use bevy_renet::{
    renet::{
        transport::{
            NetcodeServerTransport, ServerAuthentication, ServerConfig, NETCODE_KEY_BYTES,
        },
        ChannelConfig, ConnectionConfig, RenetClient, RenetServer, SendType, ServerEvent,
    },
    transport::NetcodeServerPlugin,
    RenetServerPlugin,
};
use renet_visualizer::RenetClientVisualizer;
use serde::{Deserialize, Serialize};

pub fn connection_config() -> ConnectionConfig {
    ConnectionConfig {
        available_bytes_per_tick: 1024 * 1024,
        client_channels_config: ClientChannel::channels_config(),
        server_channels_config: ServerChannel::channels_config(),
    }
}

pub const PRIVATE_KEY: &[u8; NETCODE_KEY_BYTES] = b"an example very very secret key."; // 32-bytes
pub const PROTOCOL_ID: u64 = 7;

#[derive(Debug, Component)]
pub struct Player {
    pub id: u64,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, Component, Resource)]
pub struct PlayerInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum PlayerCommand {
    BasicAttack { cast_at: Vec3 },
}

pub enum ClientChannel {
    Input,
    Command,
}

pub enum ServerChannel {
    ServerMessages,
    NetworkedEntities,
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerMessages {
    PlayerCreate {
        entity: Entity,
        id: u64,
        translation: [f32; 3],
    },
    PlayerRemove {
        id: u64,
    },
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NetworkedEntities {
    pub entities: Vec<Entity>,
    pub translations: Vec<[f32; 3]>,
    pub rotations: Vec<[f32; 4]>,
    pub wheels_translations: Vec<[[f32; 3]; 4]>,
    pub wheels_rotations: Vec<[[f32; 4]; 4]>,
}

impl From<ClientChannel> for u8 {
    fn from(channel_id: ClientChannel) -> Self {
        match channel_id {
            ClientChannel::Command => 0,
            ClientChannel::Input => 1,
        }
    }
}

impl ClientChannel {
    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            ChannelConfig {
                channel_id: Self::Input.into(),
                max_memory_usage_bytes: 5 * 1024 * 1024,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::ZERO,
                },
            },
            ChannelConfig {
                channel_id: Self::Command.into(),
                max_memory_usage_bytes: 5 * 1024 * 1024,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::ZERO,
                },
            },
        ]
    }
}

impl From<ServerChannel> for u8 {
    fn from(channel_id: ServerChannel) -> Self {
        match channel_id {
            ServerChannel::NetworkedEntities => 0,
            ServerChannel::ServerMessages => 1,
        }
    }
}

impl ServerChannel {
    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            ChannelConfig {
                channel_id: Self::NetworkedEntities.into(),
                max_memory_usage_bytes: 10 * 1024 * 1024,
                send_type: SendType::Unreliable,
            },
            ChannelConfig {
                channel_id: Self::ServerMessages.into(),
                max_memory_usage_bytes: 10 * 1024 * 1024,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
        ]
    }
}

#[derive(Debug, Default, Resource)]
pub struct ServerLobby {
    pub players: HashMap<u64, Entity>,
}

pub fn create_new_renet_server() -> (RenetServer, NetcodeServerTransport) {
    let server: RenetServer = RenetServer::new(connection_config());

    let addr = match std::env::var("APP_SERVER") {
        Ok(addr) => addr,
        _ => "0.0.0.0:5000".to_string(),
    };

    println!("Starting server on {}", addr);

    let public_addr = addr.parse().unwrap();
    let socket = UdpSocket::bind(public_addr).unwrap();
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap();

    let server_configuration = ServerConfig {
        current_time,
        max_clients: 12,
        protocol_id: PROTOCOL_ID,
        public_addresses: vec![public_addr],
        authentication: ServerAuthentication::Unsecure,
    };

    let transport = NetcodeServerTransport::new(server_configuration, socket)
        .expect("Could not set up transport server");
    return (server, transport);
}

pub fn server_update_system(
    mut server_events: EventReader<ServerEvent>,
    mut cmd: Commands,
    mut lobby: ResMut<ServerLobby>,
    mut server: ResMut<RenetServer>,
    players: Query<(Entity, &Player, &Transform)>,
    car_res: Res<bevy_garage_car::CarRes>,
    mut visualizer: ResMut<renet_visualizer::RenetServerVisualizer<200>>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                println!("Client {} connected.", client_id);

                visualizer.add_client(*client_id);

                // Send startup message to client
                server.send_message(
                    *client_id,
                    ServerChannel::ServerMessages,
                    b"Welcome!".as_slice(),
                );

                // Send information and position of all other players
                for (entity, player, transform) in players.iter() {
                    let translation: [f32; 3] = transform.translation.into();
                    let message = bincode::serialize(&ServerMessages::PlayerCreate {
                        id: player.id,
                        entity,
                        translation,
                    })
                    .unwrap();
                    server.send_message(*client_id, ServerChannel::ServerMessages, message);
                }

                // Create new player
                let transform = Transform::from_xyz(0., 0., 0.);
                let player_entity = spawn_car(
                    &mut cmd,
                    &car_res.car_scene.as_ref().unwrap(),
                    &car_res.wheel_scene.as_ref().unwrap(),
                    false,
                    transform,
                );

                cmd.entity(player_entity).insert(Player {
                    id: client_id.raw(),
                });
                lobby.players.insert(client_id.raw(), player_entity);
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                println!("Client {} disconnected for reason: {} ", client_id, reason);
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
}

pub fn update_visulizer_system(
    mut egui_contexts: bevy_egui::EguiContexts,
    mut visualizer: ResMut<renet_visualizer::RenetServerVisualizer<200>>,
    server: Res<RenetServer>,
) {
    visualizer.update(&server);
    visualizer.show_window(egui_contexts.ctx_mut());
}
