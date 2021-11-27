use bevy_egui::egui;
use bevy_egui::EguiContext;

use crate::focus::Focused;
use crate::AtariSlot;
use crate::CPU;
use crate::{system::AtariSystem, time_used_plugin::TimeUsedPlugin};
use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

#[derive(Default)]
pub struct DebugConfig {
    address: String,
}

pub fn regs(
    egui_context: Res<EguiContext>,
    mut query: Query<(&CPU, &mut AtariSystem, &AtariSlot), With<Focused>>,
    mut config: Local<DebugConfig>,
) {
    for (cpu, mut atari_system, slot) in query.iter_mut() {
        let cpu = &cpu.cpu;
        let pc = cpu.get_program_counter();

        bevy_egui::egui::Window::new("CPU").show(egui_context.ctx(), |ui| {
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
        });
        bevy_egui::egui::Window::new("ANTIC").show(egui_context.ctx(), |ui| {
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            egui::Grid::new("antic_regs").num_columns(2).show(ui, |ui| {
                let antic = &atari_system.antic;
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
        bevy_egui::egui::Window::new("GTIA").show(egui_context.ctx(), |ui| {
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

            //             " COLBK:  {:02x}  COLPF0: {:02x}  COLPF1: {:02x}  COLPF2: {:02x}  COLPF3: {:02x}  PRIOR:  {:02x}  CONSOL: {:02x} ",
            //             atari_system.gtia.regs.colors[0] as u8,
            //             atari_system.gtia.regs.colors[1] as u8,
            //             atari_system.gtia.regs.colors[2] as u8,
            //             atari_system.gtia.regs.colors[3] as u8,
            //             atari_system.gtia.regs.colors[4] as u8,
            //             atari_system.gtia.regs.prior as u8,
            //             atari_system.gtia.consol,

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

        bevy_egui::egui::Window::new("Disasm").show(egui_context.ctx(), |ui| {
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
        bevy_egui::egui::Window::new("Screen").show(egui_context.ctx(), |ui| {
            ui.add(egui::widgets::Image::new(
                egui::TextureId::User(slot.0 as u64),
                [384.0, 240.0],
            ));
        });
        bevy_egui::egui::Window::new("Memory")
            .min_width(600.0)
            .show(egui_context.ctx(), |ui| {
                let mut bytes: [u8; 256] = [0; 256];
                let addr = u16::from_str_radix(&config.address, 16).unwrap_or_default();
                atari_system.copy_to_slice(addr, &mut bytes);

                ui.text_edit_singleline(&mut config.address);
                ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
                for offs in 0..16 {
                    ui.label(format!(
                        "{:04x} {:02x?}",
                        addr as usize + offs * 16,
                        &bytes[offs * 16..(offs + 1) * 16]
                    ));
                }
            });
    }
}

pub fn frame_stats(diagnostics: Res<Diagnostics>, egui_context: Res<EguiContext>) {
    if let Some(ft) = diagnostics.get(FrameTimeDiagnosticsPlugin::FRAME_TIME) {
        if let Some(t) = diagnostics.get(TimeUsedPlugin::TIME_USED) {
            if let (Some(ft), Some(t)) = (ft.average(), t.average()) {
                bevy_egui::egui::Window::new(format!("fps: {:.1}", 1.0 / ft))
                    .id(egui::Id::new("fps"))
                    .show(egui_context.ctx(), |ui| {
                        ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
                        ui.label(format!(
                            "time: {:.1} ms {:4.1}%",
                            t * 1000.0,
                            t / ft * 100.0
                        ));
                    });
            }
        }
    }
}
