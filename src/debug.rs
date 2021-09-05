use std::time::Duration;

use crate::render_resources::SimpleMaterial;
use crate::{
    atari_text,
    render::{self, MainCamera},
    system::AtariSystem,
    time_used_plugin::TimeUsedPlugin,
    DisplayConfig,
};
use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
    render::pipeline::RenderPipeline,
};
use emulator_6502::MOS6502;

pub struct DebugComponent;
// pub struct ScanLine;
pub struct CPUDebug;
pub struct AnticDebug;
pub struct GtiaDebug;
pub struct FPS;

#[derive(Default, Bundle)]
pub struct Parent {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

pub fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<SimpleMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands
        .spawn()
        .insert_bundle(Parent {
            transform: Transform {
                translation: Vec3::new(-384.0 / 2.0, 240.0 / 2.0, 1.0),
                scale: Vec3::new(1.0, 1.0, 1.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|commands| {
            commands
                .spawn_bundle(atari_text::TextAreaBundle::new(10, 1, 0, 0))
                .insert(FPS);
        });

    commands
        .spawn()
        .insert_bundle(Parent {
            transform: Transform::from_translation(Vec3::new(384.0 / 2.0, 240.0 / 2.0, 1.0)),
            ..Default::default()
        })
        .with_children(|commands| {
            commands
                .spawn_bundle(atari_text::TextAreaBundle::new(18, 20, 0, 0))
                .insert(CPUDebug)
                .insert(DebugComponent);
            commands
                .spawn_bundle(atari_text::TextAreaBundle::new(12, 20, 18 + 1, 0))
                .insert(AnticDebug)
                .insert(DebugComponent);
            commands
                .spawn_bundle(atari_text::TextAreaBundle::new(12, 20, 18 + 12 + 2, 0))
                .insert(GtiaDebug)
                .insert(DebugComponent);
        });

    commands
        .spawn()
        .insert_bundle(Parent {
            transform: Transform::from_translation(Vec3::new(0.0, -240.0, 1.0)),
            ..Default::default()
        })
        .with_children(|commands| {
            commands
                .spawn_bundle(MeshBundle {
                    mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::new(384.0, 240.0)))),
                    render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                        render::DEBUG_COLLISIONS_PIPELINE_HANDLE.typed(),
                    )]),
                    ..Default::default()
                })
                .insert(materials.add(SimpleMaterial {
                    base_color: Color::rgba(0.0, 0.5, 0.0, 1.0),
                    base_color_texture: Some(render::COLLISIONS_TEXTURE_HANDLE.typed()),
                }))
                .insert(DebugComponent);
        });
}

#[derive(Default)]
pub struct DisplayState {
    pub last: DisplayConfig,
}

pub fn update_display_config(
    mut state: Local<DisplayState>,
    config: ResMut<DisplayConfig>,
    mut q: QuerySet<(
        Query<&mut Visible, With<FPS>>,
        Query<&mut Visible, With<DebugComponent>>,
        Query<&mut Transform, With<MainCamera>>,
    )>,
) {
    for mut camera_transform in q.q2_mut().iter_mut() {
        *camera_transform = if !config.debug {
            Transform {
                scale: Vec3::new(0.5, 0.5, 1.0),
                ..Default::default()
            }
        } else {
            Transform {
                scale: Vec3::new(1.0, 1.0, 1.0),
                translation: Vec3::new(384.0 / 2.0, -240.0 / 2.0, 0.0),
                ..Default::default()
            }
        }
    }
    if *config != state.last {
        for mut v in q.q0_mut().iter_mut() {
            v.is_visible = config.fps;
        }
        for mut v in q.q1_mut().iter_mut() {
            v.is_visible = config.debug;
        }
        state.last = *config;
    }
}

pub fn debug_overlay_system(
    display_config: ResMut<DisplayConfig>,
    mut atari_system: ResMut<AtariSystem>,
    mut q: QuerySet<(
        Query<&mut atari_text::TextArea, With<CPUDebug>>,
        Query<&mut atari_text::TextArea, With<AnticDebug>>,
        Query<&mut atari_text::TextArea, With<GtiaDebug>>,
    )>,
    // mut scan_line: Query<(&ScanLine, &mut GlobalTransform)>,
    cpu: ResMut<MOS6502>,
) {
    if !display_config.debug {
        return;
    }
    for mut text in q.q0_mut().iter_mut() {
        let mut data = vec![];
        let f = cpu.get_status_register();
        data.extend(atari_text::atascii_to_screen(
            &format!(
                " A: {:02x}   X: {:02x}     Y: {:02x}   S: {:02x}     F: {}{}-{}{}{}{}{}       {:3} / {:<3}        ",
                cpu.get_accumulator(), cpu.get_x_register(), cpu.get_y_register(), cpu.get_stack_pointer(),
                if f & 0x80 > 0 {'N'} else {'-'},
                if f & 0x40 > 0 {'V'} else {'-'},
                if f & 0x10 > 0 {'B'} else {'-'},
                if f & 0x08 > 0 {'D'} else {'-'},
                if f & 0x04 > 0 {'I'} else {'-'},
                if f & 0x02 > 0 {'Z'} else {'-'},
                if f & 0x01 > 0 {'C'} else {'-'},
                atari_system.antic.scan_line, atari_system.antic.cycle,
            ),
            false,
        ));
        data.extend(&[0; 18]);
        let pc = cpu.get_program_counter();
        let mut bytes: [u8; 48] = [0; 48];
        atari_system.copy_to_slice(pc, &mut bytes);
        if let Ok(instructions) = disasm6502::from_addr_array(&bytes, pc) {
            for i in instructions.iter().take(16) {
                let line = format!(" {:04x} {:11} ", i.address, i.as_str());
                data.extend(atari_text::atascii_to_screen(&line, i.address == pc));
            }
        }
        text.data.data[..data.len()].copy_from_slice(&data);
    }

    for mut text in q.q1_mut().iter_mut() {
        let status_text = format!(
            " IR: {:02x}      DMACTL: {:02x}  CHBASE: {:02x}  HSCROL: {:02x}  VSCROL: {:02x}  PMBASE: {:02x}  VCOUNT: {:02x}  NMIST:  {:02x}  NMIEN:  {:02x} ",
            atari_system.antic.ir(),
            atari_system.antic.dmactl.bits(),
            atari_system.antic.chbase,
            atari_system.antic.hscrol,
            atari_system.antic.vscrol,
            atari_system.antic.pmbase,
            atari_system.antic.vcount,
            atari_system.antic.nmist,
            atari_system.antic.nmien,
        );
        let data = atari_text::atascii_to_screen(&status_text, false);
        text.data.data[..data.len()].copy_from_slice(&data);
    }
    for mut text in q.q2_mut().iter_mut() {
        let status_text = format!(
            " COLBK:  {:02x}  COLPF0: {:02x}  COLPF1: {:02x}  COLPF2: {:02x}  COLPF3: {:02x}  PRIOR:  {:02x}  CONSOL: {:02x} ",
            atari_system.gtia.regs.colors[0] as u8,
            atari_system.gtia.regs.colors[1] as u8,
            atari_system.gtia.regs.colors[2] as u8,
            atari_system.gtia.regs.colors[3] as u8,
            atari_system.gtia.regs.colors[4] as u8,
            atari_system.gtia.regs.prior as u8,
            atari_system.gtia.consol,
        );
        let data = atari_text::atascii_to_screen(&status_text, false);
        text.data.data[..data.len()].copy_from_slice(&data);
    }
    // for (_, mut transform) in scan_line.iter_mut() {
    //     *transform = GlobalTransform::from_translation(Vec3::new(
    //         0.0,
    //         128.0 - atari_system.antic.scan_line as f32,
    //         0.1,
    //     ))
    //     .mul_transform(Transform::from_scale(Vec3::new(384.0, 1.0, 1.0)));
    // }
}

pub struct FPSState(Timer);

impl Default for FPSState {
    fn default() -> Self {
        FPSState(Timer::new(Duration::from_secs(1), true))
    }
}

pub fn update_fps(
    mut state: Local<FPSState>,
    time: Res<Time>,
    mut fps_query: Query<&mut atari_text::TextArea, With<FPS>>,
    diagnostics: Res<Diagnostics>,
) {
    if state.0.tick(time.delta()).finished() {
        for mut fps in fps_query.iter_mut() {
            if let Some(ft) = diagnostics.get(FrameTimeDiagnosticsPlugin::FRAME_TIME) {
                if let Some(t) = diagnostics.get(TimeUsedPlugin::TIME_USED) {
                    if let (Some(ft), Some(t)) = (ft.average(), t.average()) {
                        info!("fps: {:.1} {:.2}", 1.0 / ft, t / ft);
                        fps.set_text(&format!("{:.1} {:.2}", 1.0 / ft, t / ft));
                    }
                }
            }
        }
    }
}
