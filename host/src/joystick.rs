use crate::CarSet;
use bevy::prelude::*;
use bevy_garage_car::{Car, Player};
use virtual_joystick::*;

#[derive(Default, Reflect, Hash, Clone, PartialEq, Eq)]
enum JoystickTypeAxis {
    #[default]
    X,
    Y,
}

pub struct CarJoystickPlugin;
impl Plugin for CarJoystickPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(VirtualJoystickPlugin::<JoystickTypeAxis>::default())
            .add_systems(Startup, joystick_start_system)
            .add_systems(Update, update_joystick.in_set(CarSet::Input));
    }
}

const MARGIN: Val = Val::Px(35.);
const KNOB_SIZE: Vec2 = Vec2::new(70., 70.);
const AREA_SIZE: Val = Val::Px(150.);
const BG: Color = Color::rgba(1.0, 0.27, 0.0, 0.1);

pub fn joystick_start_system(mut cmd: Commands, asset_server: Res<AssetServer>) {
    create_joystick(
        &mut cmd,
        asset_server.load("joystick/Outline.png"),
        asset_server.load("joystick/Horizontal_Outline_Arrows.png"),
        None,
        None,
        Some(BG),
        KNOB_SIZE,
        Vec2::new(150., 150.),
        VirtualJoystickNode {
            dead_zone: 0.,
            id: JoystickTypeAxis::X,
            axis: VirtualJoystickAxis::Horizontal,
            behaviour: VirtualJoystickType::Fixed,
        },
        Style {
            width: AREA_SIZE,
            height: AREA_SIZE,
            position_type: PositionType::Absolute,
            left: MARGIN,
            bottom: MARGIN,
            ..default()
        },
    );

    create_joystick(
        &mut cmd,
        asset_server.load("joystick/Outline.png"),
        asset_server.load("joystick/Vertical_Outline_Arrows.png"),
        None,
        None,
        Some(BG),
        KNOB_SIZE,
        Vec2::new(150., 150.),
        VirtualJoystickNode {
            dead_zone: 0.,
            id: JoystickTypeAxis::Y,
            axis: VirtualJoystickAxis::Vertical,
            behaviour: VirtualJoystickType::Fixed,
        },
        Style {
            width: AREA_SIZE,
            height: AREA_SIZE,
            position_type: PositionType::Absolute,
            right: MARGIN,
            bottom: MARGIN,
            ..default()
        },
    );
}

fn update_joystick(
    mut virtual_joystick_events: EventReader<VirtualJoystickEvent<JoystickTypeAxis>>,
    mut cars: Query<&mut Car, With<Player>>,
) {
    for mut car in cars.iter_mut() {
        for j in virtual_joystick_events.read() {
            let Vec2 { x, y } = j.axis();
            // println!("x{x}, y{y}");
            match j.id() {
                JoystickTypeAxis::X => {
                    car.steering = x;
                }
                JoystickTypeAxis::Y => {
                    if y < 0. {
                        car.brake = -y;
                        car.gas = 0.;
                    } else {
                        car.gas = y;
                        car.brake = 0.;
                    }
                }
            }
        }
    }
}
