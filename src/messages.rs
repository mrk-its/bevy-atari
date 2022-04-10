use crate::{gdb::GdbMessage, AtariSlot, AtariSystem, BreakPoint, Debugger, EmulatorState, CPU};
use bevy::prelude::*;
use emulator_6502::Interface6502;
use gdbstub_mos_arch::MosRegs;
use once_cell::sync::Lazy;
use parking_lot::RwLock;

#[allow(dead_code)]
pub static MESSAGES: Lazy<RwLock<Vec<Message>>> = Lazy::new(|| RwLock::new(vec![]));

#[derive(Clone, Debug)]
pub enum Message {
    JoyState {
        port: usize,
        dirs: u8,
        fire: bool,
    },
    SetConsol {
        state: u8,
    },
    BinaryData {
        key: String,
        path: String,
        data: Option<Vec<u8>>,
        slot: Option<i32>,
    },
    Command {
        cmd: String,
    },
    Reset {
        cold: bool,
        disable_basic: Option<bool>,
    },
    SetState(String),
    SetResolution {
        width: f32,
        height: f32,
    },
    ClearBreakpoints,
    AddBreakpoint(BreakPoint),
    DelBreakpoint(BreakPoint),
    ReadRegisters,
    ReadMemory(u16, u16),
    SingleStep,
    Pause,
    Continue,
    KeyStrokes {
        text: String,
    },
}

pub fn send_message(msg: Message) {
    let mut messages = MESSAGES.write();
    messages.push(msg);
}

pub fn events(
    mut query: Query<(&AtariSlot, &mut AtariSystem, &mut CPU, &mut Debugger)>,
    mut state: ResMut<State<EmulatorState>>,
    mut windows: ResMut<Windows>,
    mut ui_config: ResMut<crate::resources::UIConfig>,
) {
    let mut _messages = MESSAGES.write();
    for (atari_slot, mut atari_system, mut cpu, mut debugger) in query.iter_mut() {
        let mut messages = (*_messages).clone();
        for event in messages.drain(..) {
            match event {
                Message::Continue => {
                    debugger.cont();
                }
                Message::Pause => {
                    debugger.pause();
                }
                Message::SingleStep => {
                    debugger.step_into();
                }
                Message::ReadRegisters => {
                    let mut regs: MosRegs = Default::default();
                    regs.pc = cpu.cpu.get_program_counter();
                    regs.flags = cpu.cpu.get_status_register();
                    regs.a = cpu.cpu.get_accumulator();
                    regs.x = cpu.cpu.get_x_register();
                    regs.y = cpu.cpu.get_y_register();
                    regs.s = cpu.cpu.get_stack_pointer();
                    atari_system.copy_to_slice(0xcb, &mut regs.rc);
                    debugger.send_message(GdbMessage::Registers(regs));
                }
                Message::ReadMemory(offs, len) => {
                    let mut data = vec![0; len as usize];
                    atari_system.copy_to_slice(offs, &mut data);
                    debugger.send_message(GdbMessage::Memory(offs, data));
                }
                Message::AddBreakpoint(bp) => {
                    debugger.add_breakpoint(bp);
                }
                Message::DelBreakpoint(bp) => {
                    debugger.del_breakpoint(bp);
                }
                Message::ClearBreakpoints => {
                    debugger.clear_breakpoints();
                }
                Message::SetResolution { width, height } => {
                    let window = windows.get_primary_mut().unwrap();
                    window.set_resolution(width, height);
                }
                Message::Reset {
                    cold,
                    disable_basic,
                } => {
                    atari_system.reset(
                        &mut cpu.cpu,
                        cold,
                        disable_basic.unwrap_or(!ui_config.basic),
                    );
                    debugger.paused = false;
                }
                Message::SetState(new_state) => {
                    let _result = match new_state.as_ref() {
                        "running" => state.set(EmulatorState::Running),
                        "idle" => state.set(EmulatorState::Idle),
                        _ => panic!("invalid state requested"),
                    }
                    .ok()
                    .is_some();
                }
                Message::JoyState { port, dirs, fire } => {
                    atari_system.set_joystick(1, port, dirs, fire)
                }
                Message::SetConsol { state } => {
                    atari_system.update_consol(1, state);
                }
                Message::BinaryData {
                    key,
                    data,
                    slot,
                    path,
                } => {
                    if slot.is_none() || Some(atari_slot.0) == slot {
                        let data = match data.as_ref() {
                            Some(data) => Some(&data[..]),
                            None => None,
                        };
                        if key == "basic" {
                            ui_config.basic = data.is_some();
                        }
                        crate::set_binary(&mut atari_system, &mut cpu, &key, &path, data);
                    }
                }
                Message::KeyStrokes { text } => {
                    atari_system.keystrokes(&text);
                }
                Message::Command { cmd } => {
                    let parts = cmd.split(" ").collect::<Vec<_>>();
                    match parts[0] {
                        "mem" => {
                            if let Ok(start) = u16::from_str_radix(parts[1], 16) {
                                let mut data = [0 as u8; 256];
                                atari_system.copy_to_slice(start, &mut data);
                                info!("{:x?}", data);
                            }
                        }
                        "write" => {
                            if let Ok(addr) = u16::from_str_radix(parts[1], 16) {
                                if let Ok(value) = u8::from_str_radix(parts[2], 16) {
                                    atari_system.write(addr, value);
                                    info!("write {:04x} <- {:02x}", addr, value);
                                }
                            }
                        }
                        "pc" => {
                            if let Ok(pc) = u16::from_str_radix(parts[1], 16) {
                                cpu.cpu.set_program_counter(pc)
                            }
                        }
                        "brk" => {
                            if let Ok(pc) = u16::from_str_radix(parts[1], 16) {
                                debugger.breakpoints.push(BreakPoint::PC(pc));
                                info!("breakpoint set on pc={:04x}", pc);
                            }
                        }
                        "trainer_init" => {
                            atari_system.trainer_init();
                        }
                        "trainer_changed" => {
                            let cnt = atari_system.trainer_changed(true);
                            info!("matched: {}", cnt);
                        }
                        "trainer_unchanged" => {
                            let cnt = atari_system.trainer_changed(false);
                            info!("matched: {}", cnt);
                        }
                        _ => (),
                    }
                }
            }
        }
    }
    _messages.clear();
}
