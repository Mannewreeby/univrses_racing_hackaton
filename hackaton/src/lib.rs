use std::time::Duration;

use bevy::{
    asset::Assets, math::Vec3, pbr::StandardMaterial, prelude::{Commands, Component, Entity, Event, Mesh, ResMut, Resource, Transform}, utils::HashMap
};
use bevy_rapier3d::prelude::{Collider, ColliderScale, CollisionGroups, RigidBody};
use bevy_renet::renet::{ChannelConfig, ConnectionConfig, DisconnectReason, SendType};
use serde::{Deserialize, Serialize};

pub mod shared_systems;

#[derive(Debug, Component)]
pub struct Player {
    pub id: u64,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, Component, Resource)]
pub struct PlayerInput {
    pub forward: bool,
    pub left: bool,
    pub right: bool,
    pub brake: bool,
}

pub enum ClientChannel {
    Input,
}

impl From<ClientChannel> for u8 {
    fn from(channel_id: ClientChannel) -> Self {
        match channel_id {
            ClientChannel::Input => 1,
        }
    }
}

impl ClientChannel {
    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![ChannelConfig {
            channel_id: Self::Input.into(),
            max_memory_usage_bytes: 5 * 1024 * 1024,
            send_type: SendType::ReliableOrdered {
                resend_time: Duration::ZERO,
            },
            // Potential user attack info goes here
        }]
    }
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
        position: [f32; 3],
    },
    PlayerRemove {
        id: u64,
    },
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

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NetworkedEntities {
    pub entities: Vec<Entity>,
    pub positions: Vec<[f32; 3]>,
    pub orientations: Vec<[f32; 4]>,
    pub wheel_positions: Vec<[[f32; 3]; 4]>,
    pub wheen_orientations: Vec<[[f32; 4]; 4]>,
}

#[derive(Debug, Default, Resource)]
pub struct ServerLobby {
    pub players: HashMap<u64, Entity>,
}

pub fn connection_config() -> ConnectionConfig {
    ConnectionConfig {
        available_bytes_per_tick: 1024 * 1024,
        client_channels_config: ClientChannel::channels_config(),
        server_channels_config: ServerChannel::channels_config(),
    }
}

pub const SERVER_PROTOCOL_ID: u64 = 7;
#[derive(Debug, PartialEq, Eq, Event)]
pub enum ServerEvent {
    ClientConnected {
        client_id: u64,
    },
    ClientDisconnected {
        client_id: u64,
        reason: DisconnectReason,
    },
}


