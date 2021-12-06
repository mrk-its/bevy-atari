use bevy::input::mouse::MouseMotion;
use bevy_egui::egui;
use bevy_egui::egui::InnerResponse;
use bevy_egui::EguiContext;

use crate::focus::Focused;
use crate::resources::UIConfig;
use crate::AtariSlot;
use crate::Debugger;
use crate::CPU;
use crate::{system::AtariSystem, time_used_plugin::TimeUsedPlugin};

use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

type Response = Option<InnerResponse<Option<()>>>;

fn show_config(egui_context: &EguiContext, config: &mut UIConfig) -> Response {
    bevy_egui::egui::Window::new("Config")
        .anchor(egui::Align2::LEFT_TOP, egui::Vec2::new(16.0, 16.0))
        .show(egui_context.ctx(), |ui| {
            ui.checkbox(&mut config.cpu, "CPU");
            ui.checkbox(&mut config.antic, "ANTIC");
            ui.checkbox(&mut config.gtia, "GTIA");
            ui.checkbox(&mut config.small_screen, "Screen Window");
            ui.checkbox(&mut config.memory[0].enabled, "Memory1");
            ui.checkbox(&mut config.memory[1].enabled, "Memory2");
            ui.checkbox(&mut config.memory[2].enabled, "Memory3");
            ui.checkbox(&mut config.memory[3].enabled, "Memory4");
            ui.checkbox(&mut config.disasm, "Disassembler");
            ui.checkbox(&mut config.debugger, "Debugger");
        })
}
fn show_debugger(
    egui_context: &EguiContext,
    config: &mut UIConfig,
    debugger: &mut Debugger,
    cpu: &CPU,
    system: &AtariSystem,
) -> Response {
    bevy_egui::egui::Window::new("Debugger")
        .open(&mut config.debugger)
        .show(egui_context.ctx(), |ui| {
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            if ui.button(if debugger.paused {"Resume (F6)"} else {"Pause (F6)"}).clicked() {
                debugger.paused = !debugger.paused;
            }
            // ui.checkbox(&mut debugger.paused, "pause (F6)");
            if ui.button("step into (F7)").clicked() {
                debugger.step_into();
            }
            if ui.button("step over (F8)").clicked() {
                debugger.step_over(&cpu.cpu);
            }
            if ui.button("next scanline (F9)").clicked() {
                debugger.next_scanline(&system.antic)
            }
            if ui.button("next frame (F10)").clicked() {
                debugger.next_frame()
            }
        })
}

fn show_cpu(egui_context: &EguiContext, config: &mut UIConfig, cpu: &CPU) -> Response {
    let cpu = &cpu.cpu;
    let pc = cpu.get_program_counter();
    bevy_egui::egui::Window::new("CPU")
        .open(&mut config.cpu)
        .show(egui_context.ctx(), |ui| {
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            let flags = cpu.get_status_register();
            let flags_str = "NV-BDIZC"
                .chars()
                .enumerate()
                .map(|(i, c)| if flags & (0x80 >> i) > 0 { c } else { '-' })
                .collect::<String>();

            egui::Grid::new("cpu_regs").num_columns(2).show(ui, |ui| {
                ui.label("PC");
                ui.label(format!("{:04x}", pc));
                ui.end_row();
                ui.label("F");
                ui.label(flags_str);
                ui.end_row();
                ui.label("S");
                ui.label(format!("{:02x}", cpu.get_stack_pointer()));
                ui.end_row();
                ui.label("A");
                ui.label(format!("{:02x}", cpu.get_accumulator()));
                ui.end_row();
                ui.label("X");
                ui.label(format!("{:02x}", cpu.get_x_register()));
                ui.end_row();
                ui.label("Y");
                ui.label(format!("{:02x}", cpu.get_y_register()));
                ui.end_row();
            });
        })
}

fn show_antic(egui_context: &EguiContext, config: &mut UIConfig, atari_system: &mut AtariSystem) {
    bevy_egui::egui::Window::new("ANTIC")
        .open(&mut config.antic)
        .show(egui_context.ctx(), |ui| {
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            egui::Grid::new("antic_regs").num_columns(2).show(ui, |ui| {
                let antic = &atari_system.antic;
                ui.label("scanline");
                ui.label(format!("{}", antic.scan_line));
                ui.label("DLIST");
                ui.label(format!("{:04x}", antic.dlist));
                ui.end_row();
                ui.label("IR");
                ui.label(format!("{:02x}", antic.ir()));
                ui.end_row();
                ui.label("DMACTL");
                ui.label(format!("{:02x}", antic.dmactl.bits()));
                ui.end_row();
                ui.label("CHBASE");
                ui.label(format!("{:02x}", antic.chbase));
                ui.end_row();
                ui.label("HSCROL");
                ui.label(format!("{:02x}", antic.hscrol));
                ui.end_row();
                ui.label("VSCROL");
                ui.label(format!("{:02x}", antic.vscrol));
                ui.end_row();
                ui.label("PMBASE");
                ui.label(format!("{:02x}", antic.pmbase));
                ui.end_row();
                ui.label("VCOUNT");
                ui.label(format!("{:02x}", antic.pmbase));
                ui.end_row();
                ui.label("NMIST");
                ui.label(format!("{:02x}", antic.nmist));
                ui.end_row();
                ui.label("NMIEN");
                ui.label(format!("{:02x}", antic.nmien));
                ui.end_row();
            });
        });
}

fn show_gtia(egui_context: &EguiContext, config: &mut UIConfig, atari_system: &mut AtariSystem) {
    bevy_egui::egui::Window::new("GTIA")
        .open(&mut config.gtia)
        .show(egui_context.ctx(), |ui| {
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

            egui::Grid::new("gtia_regs").num_columns(4).show(ui, |ui| {
                let gtia = &atari_system.gtia;

                ui.label("HPOSP0");
                ui.label(format!("{:02x}", gtia.regs.hposp[0]));

                ui.label("COLPM0");
                ui.label(format!("{:02x}", gtia.regs.col[0]));
                ui.end_row();

                ui.label("HPOSP1");
                ui.label(format!("{:02x}", gtia.regs.hposp[1]));

                ui.label("COLPM1");
                ui.label(format!("{:02x}", gtia.regs.col[1]));
                ui.end_row();

                ui.label("HPOSP2");
                ui.label(format!("{:02x}", gtia.regs.hposp[2]));

                ui.label("COLPM2");
                ui.label(format!("{:02x}", gtia.regs.col[2]));
                ui.end_row();

                ui.label("HPOSP3");
                ui.label(format!("{:02x}", gtia.regs.hposp[3]));

                ui.label("COLPM3");
                ui.label(format!("{:02x}", gtia.regs.col[3]));
                ui.end_row();

                ui.label("SIZEP0");
                ui.label(format!("{:02x}", gtia.regs.sizep[0]));

                ui.label("COLPF0");
                ui.label(format!("{:02x}", gtia.regs.col[4]));
                ui.end_row();

                ui.label("SIZEP1");
                ui.label(format!("{:02x}", gtia.regs.sizep[1]));

                ui.label("COLPF1");
                ui.label(format!("{:02x}", gtia.regs.col[5]));
                ui.end_row();

                ui.label("SIZEP2");
                ui.label(format!("{:02x}", gtia.regs.sizep[2]));

                ui.label("COLPF2");
                ui.label(format!("{:02x}", gtia.regs.col[6]));
                ui.end_row();

                ui.label("SIZEP3");
                ui.label(format!("{:02x}", gtia.regs.sizep[3]));

                ui.label("COLPF3");
                ui.label(format!("{:02x}", gtia.regs.col[7]));
                ui.end_row();

                ui.label("SIZEM");
                ui.label(format!("{:02x}", gtia.regs.sizem));

                ui.label("COLBK");
                ui.label(format!("{:02x}", gtia.regs.col[8]));
                ui.end_row();

                ui.separator();
                ui.end_row();

                ui.label("GRAFP0");
                ui.label(format!("{:02x}", gtia.regs.grafp[0]));
                ui.label("PRIOR");
                ui.label(format!("{:02x}", gtia.regs.prior));
                ui.end_row();

                ui.label("GRAFP1");
                ui.label(format!("{:02x}", gtia.regs.grafp[1]));
                ui.label("VDELAY");
                ui.label(format!("{:02x}", gtia.regs.vdelay));
                ui.end_row();

                ui.label("GRAFP2");
                ui.label(format!("{:02x}", gtia.regs.grafp[2]));
                ui.label("GRACTL");
                ui.label(format!("{:02x}", gtia.regs.gractl));
                ui.end_row();

                ui.label("GRAFP3");
                ui.label(format!("{:02x}", gtia.regs.grafp[3]));
                ui.label("CONSOL");
                ui.label(format!("{:02x}", gtia.regs.consol));
                ui.end_row();

                ui.label("GRAFM");
                ui.label(format!("{:02x}", gtia.regs.grafm));
            });
        });
}

fn show_disasm(
    egui_context: &EguiContext,
    config: &mut UIConfig,
    cpu: &CPU,
    atari_system: &mut AtariSystem,
) {
    let pc = cpu.cpu.get_program_counter();
    bevy_egui::egui::Window::new("Disassembler")
        .open(&mut config.disasm)
        .show(egui_context.ctx(), |ui| {
            let mut bytes: [u8; 48] = [0; 48];
            atari_system.copy_to_slice(pc, &mut bytes);
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            ui.vertical(|ui| {
                if let Ok(instructions) = disasm6502::from_addr_array(&bytes, pc) {
                    for i in instructions.iter().take(16) {
                        let line = format!(" {:04x} {:11} ", i.address, i.as_str());
                        let mut label = egui::Label::new(line);
                        if i.address == pc {
                            label = label
                                .background_color(egui::Color32::from_rgb(64, 64, 64))
                                .text_color(egui::Color32::WHITE);
                        }

                        ui.add(label);
                    }
                }
            });
        });
}

fn show_memory(
    egui_context: &EguiContext,
    index: usize,
    config: &mut UIConfig,
    atari_system: &mut AtariSystem,
) {
    let mem_config = &mut config.memory[index];
    bevy_egui::egui::Window::new(format!("Memory{}", index + 1))
        .open(&mut mem_config.enabled)
        .min_width(600.0)
        .show(egui_context.ctx(), |ui| {
            let addr = u16::from_str_radix(&mem_config.address, 16).unwrap_or_default();
            let mut bytes: [u8; 256] = [0; 256];
            atari_system.copy_to_slice(addr, &mut bytes);
            let hex = bytes
                .iter()
                .map(|b| format!(" {:02x}", b))
                .collect::<Vec<_>>()
                .join("");

            ui.text_edit_singleline(&mut mem_config.address);
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            for offs in 0..16 {
                ui.label(format!(
                    "{:04x}{}",
                    addr as usize + offs * 16,
                    &hex[offs * 16 * 3..(offs + 1) * 16 * 3]
                ));
            }
        });
}

fn show_screen(egui_context: &EguiContext, config: &mut UIConfig, slot: &AtariSlot) {
    bevy_egui::egui::Window::new("Screen")
        .open(&mut config.small_screen)
        .show(egui_context.ctx(), |ui| {
            ui.add(egui::widgets::Image::new(
                egui::TextureId::User(slot.0 as u64),
                [384.0 * 2.0, 240.0 * 2.0],
            ));
        });
}

fn show_fps(
    egui_context: &EguiContext,
    config: &mut UIConfig,
    diagnostics: &Diagnostics,
) -> Response {
    if !config.fps {
        return None;
    }
    if let Some(ft) = diagnostics.get(FrameTimeDiagnosticsPlugin::FRAME_TIME) {
        if let Some(t) = diagnostics.get(TimeUsedPlugin::TIME_USED) {
            if let (Some(ft), Some(t)) = (ft.average(), t.average()) {
                return bevy_egui::egui::Window::new(format!("fps: {:.1}", 1.0 / ft))
                    .anchor(egui::Align2::RIGHT_TOP, egui::Vec2::new(-16.0, 16.0))
                    .id(egui::Id::new("fps"))
                    .show(egui_context.ctx(), |ui| {
                        ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
                        ui.label(format!(
                            "time: {:4.1} ms {:4.1}%",
                            t * 1000.0,
                            t / ft * 100.0
                        ));
                    });
            }
        }
    }
    return None;
}

pub fn show_ui(
    egui_context: Res<EguiContext>,
    diagnostics: Res<Diagnostics>,
    mut query: Query<(&CPU, &mut AtariSystem, &AtariSlot, &mut Debugger), With<Focused>>,
    mut config: ResMut<UIConfig>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    windows: Res<Windows>,
) {
    let window = windows.get_primary().unwrap();
    let cursor_pos = (window.height() - window.cursor_position().unwrap_or_default().y).abs();
    let mouse_moved = cursor_pos < 100.0 && mouse_motion_events.iter().count() > 0;
    let show_ui = if mouse_moved {
        config.reset_auto_hide()
    } else {
        config.auto_hide_tick()
    };

    if !show_ui {
        return;
    }

    fn force_show(response: &Response) -> bool {
        match response {
            Some(InnerResponse { inner: None, .. }) | None => false,
            Some(InnerResponse {
                inner: Some(_),
                response: r,
            }) => r.hovered(),
        }
    }

    let r1 = show_fps(&egui_context, &mut config, &diagnostics);
    let r2 = show_config(&egui_context, &mut config);

    let is_collapsed = if let Some(InnerResponse { inner: None, .. }) = r2 {
        true
    } else {
        false
    };

    if !(force_show(&r1) || force_show(&r2)) && config.all_unchecked() {
        return;
    }
    if is_collapsed {
        return;
    }
    config.reset_auto_hide();
    for (cpu, mut atari_system, slot, mut debugger) in query.iter_mut() {
        show_screen(&egui_context, &mut config, slot);
        show_cpu(&egui_context, &mut config, &cpu);
        show_debugger(&egui_context, &mut config, &mut debugger, &cpu, &atari_system);
        show_antic(&egui_context, &mut config, &mut atari_system);
        show_gtia(&egui_context, &mut config, &mut atari_system);
        show_disasm(&egui_context, &mut config, cpu, &mut atari_system);
        for index in 0..4 {
            show_memory(&egui_context, index, &mut config, &mut atari_system);
        }
    }
}
