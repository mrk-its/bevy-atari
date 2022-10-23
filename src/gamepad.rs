use bevy::prelude::*;

use bevy::prelude::GamepadButtonType;

use crate::focus::Focused;
use crate::system::AtariSystem;

pub fn update(
    mut query: Query<&mut AtariSystem, With<Focused>>,
    gamepad_buttons: Res<Input<GamepadButton>>,
    axis: Res<Axis<GamepadAxis>>,
) {
    for mut atari_system in query.iter_mut() {
        let mut consol = 0;
        let axis_threshold = 0.5;
        for idx in 0..2 {
            let pad = Gamepad::new(idx);
            let stick_x = axis
                .get(GamepadAxis::new(pad, GamepadAxisType::LeftStickX))
                .unwrap_or_default();
            let stick_y = axis
                .get(GamepadAxis::new(pad, GamepadAxisType::LeftStickY))
                .unwrap_or_default();

            let up = gamepad_buttons.pressed(GamepadButton::new(pad, GamepadButtonType::DPadUp))
                || stick_y >= axis_threshold;
            let down = gamepad_buttons
                .pressed(GamepadButton::new(pad, GamepadButtonType::DPadDown))
                || stick_y <= -axis_threshold;
            let left = gamepad_buttons
                .pressed(GamepadButton::new(pad, GamepadButtonType::DPadLeft))
                || stick_x <= -axis_threshold;
            let right = gamepad_buttons
                .pressed(GamepadButton::new(pad, GamepadButtonType::DPadRight))
                || stick_x >= axis_threshold;
            let dirs = up as u8 | down as u8 * 2 | left as u8 * 4 | right as u8 * 8;
            let fire = gamepad_buttons.pressed(GamepadButton::new(pad, GamepadButtonType::East))
                || gamepad_buttons.pressed(GamepadButton::new(pad, GamepadButtonType::LeftTrigger))
                || gamepad_buttons
                    .pressed(GamepadButton::new(pad, GamepadButtonType::RightTrigger));

            atari_system.set_joystick(0, idx, dirs, fire);
            consol |= gamepad_buttons.pressed(GamepadButton::new(pad, GamepadButtonType::South))
                as u8
                + gamepad_buttons.pressed(GamepadButton::new(pad, GamepadButtonType::North)) as u8
                    * 2
                + gamepad_buttons.pressed(GamepadButton::new(pad, GamepadButtonType::West)) as u8
                    * 4;
        }
        atari_system.update_consol(1, consol);
    }
}
