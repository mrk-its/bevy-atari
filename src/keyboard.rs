use bevy::prelude::*;
use emulator_6502::MOS6502;
use std::time::Duration;

use bevy::prelude::{GamepadButtonType, KeyCode};

use crate::system::Interface6502;
use crate::{system::AtariSystem, BreakPoint, DisplayConfig, FrameState};

#[derive(Default)]
pub struct KeyboarSystemState {
    timer: Timer,
}

pub fn system(
    time: Res<Time>,
    mut display_config: ResMut<DisplayConfig>,
    keyboard: Res<Input<KeyCode>>,
    gamepad_buttons: Res<Input<GamepadButton>>,
    axis: Res<Axis<GamepadAxis>>,
    mut state: Local<KeyboarSystemState>,
    mut frame: ResMut<FrameState>,
    mut atari_system: ResMut<AtariSystem>,
    cpu: ResMut<MOS6502>,
) {
    if state.timer.finished() {
        if keyboard.just_pressed(KeyCode::F7) {
            display_config.fps = !display_config.fps;
        } else if keyboard.just_pressed(KeyCode::F8) {
            display_config.debug = !display_config.debug;
        } else if keyboard.pressed(KeyCode::F9) {
            if !frame.paused {
                frame.set_breakpoint(BreakPoint::ScanLine(248))
            } else {
                // frame.break_point = None;
                frame.paused = false;
            }
        } else if keyboard.pressed(KeyCode::F10) {
            let next_scan_line = atari_system.antic.get_next_scanline();
            frame.set_breakpoint(BreakPoint::ScanLine(next_scan_line));
        } else if keyboard.pressed(KeyCode::F11) {
            if atari_system.read(cpu.get_program_counter()) == 0x20 {
                // JSR
                frame.set_breakpoint(BreakPoint::PC(cpu.get_program_counter() + 3));
            } else {
                frame.set_breakpoint(BreakPoint::NotPC(cpu.get_program_counter()));
            }
        } else if keyboard.pressed(KeyCode::F12) {
            frame.set_breakpoint(BreakPoint::NotPC(cpu.get_program_counter()));
        }
    }
    for _ in keyboard.get_just_pressed() {
        state.timer.set_duration(Duration::from_secs_f32(0.2));
        state.timer.set_repeating(false);
        state.timer.reset();
        break;
    }
    for _ in keyboard.get_just_released() {
        state.timer.set_duration(Duration::default());
        state.timer.reset();
        break;
    }
    state.timer.tick(time.delta());

    let mut consol = 0;
    let axis_threshold = 0.5;
    for idx in 0..2 {
        let pad = Gamepad(idx);
        let stick_x = axis
            .get(GamepadAxis(pad, GamepadAxisType::LeftStickX))
            .unwrap_or_default();
        let stick_y = axis
            .get(GamepadAxis(pad, GamepadAxisType::LeftStickY))
            .unwrap_or_default();

        let up = gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::DPadUp))
            || stick_y >= axis_threshold;
        let down = gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::DPadDown))
            || stick_y <= -axis_threshold;
        let left = gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::DPadLeft))
            || stick_x <= -axis_threshold;
        let right = gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::DPadRight))
            || stick_x >= axis_threshold;
        let dirs = up as u8 | down as u8 * 2 | left as u8 * 4 | right as u8 * 8;
        let fire = gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::East))
            || gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::LeftTrigger))
            || gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::RightTrigger));

        atari_system.set_joystick(0, idx, dirs, fire);
        consol |= gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::South)) as u8
            + gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::North)) as u8 * 2
            + gamepad_buttons.pressed(GamepadButton(pad, GamepadButtonType::West)) as u8 * 4;
    }
    atari_system.update_consol(1, consol);
}
