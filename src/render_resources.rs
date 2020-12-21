use crate::gtia::atari_color;
use bevy::asset::Handle;
use bevy::core::{Byteable, Bytes};
use bevy::prelude::Color;
use bevy::reflect::TypeUuid;
use bevy::render::{
    impl_render_resource_bytes,
    renderer::{RenderResource, RenderResourceType},
    texture::Texture,
};
use bevy::{prelude::*, render::renderer::RenderResources};
use std::convert::TryInto;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Charset {
    pub data: [u8; 1024],
}

impl Default for Charset {
    fn default() -> Self {
        Self {
            data: [0; 1024],
        }
    }
}

impl Charset {
    pub fn new(src: &[u8]) -> Self {
        Self {
            data: src.try_into().expect("byte slice of length 1024"),
        }
    }
}

unsafe impl Byteable for Charset {}
impl_render_resource_bytes!(Charset);

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct LineData {
    pub data: [u8; 48],
    pub player0: [u8; 16],
    pub player1: [u8; 16],
    pub player2: [u8; 16],
    pub player3: [u8; 16],
}
impl Default for LineData {
    fn default() -> Self {
        Self {
            data: [0; 48],
            player0: [0; 16],
            player1: [0; 16],
            player2: [0; 16],
            player3: [0; 16],
        }
    }
}
impl LineData {
    pub fn new(src: &[u8], player0: &[u8], player1: &[u8], player2: &[u8], player3: &[u8]) -> Self {
        Self {
            data: src.try_into().expect("byte slice of length 48"),
            player0: player0.try_into().expect("slice of 16 bytes"),
            player1: player1.try_into().expect("slice of 16 bytes"),
            player2: player2.try_into().expect("slice of 16 bytes"),
            player3: player3.try_into().expect("slice of 16 bytes"),
        }
    }
}

unsafe impl Byteable for LineData {}
impl_render_resource_bytes!(LineData);

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub data: [Color; 256],
}

impl Default for Palette {
    fn default() -> Self {
        let palette: Vec<_> = (0..=255).map(|index| atari_color(index)).collect();
        Self {
            data: palette
                .as_slice()
                .try_into()
                .expect("byte slice of length 256"),
        }
    }
}

unsafe impl Byteable for Palette {}
impl_render_resource_bytes!(Palette);

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct GTIARegsArray {
    pub regs: [GTIARegs; 8],
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct GTIARegs {
    pub colors: [u32; 8],
    pub colors_pm: [u32; 4],
    pub player_pos: [f32; 4],
    pub player_size: [f32; 4],
    pub prior: u32,
    pub _fill: [u32; 3],
}

impl GTIARegs {
    pub fn new(
        colbk: u8,
        colpf0: u8,
        colpf1: u8,
        colpf2: u8,
        colpf3: u8,
        colpm0: u8,
        colpm1: u8,
        colpm2: u8,
        colpm3: u8,
        hposp0: u8,
        hposp1: u8,
        hposp2: u8,
        hposp3: u8,
        sizep0: u8,
        sizep1: u8,
        sizep2: u8,
        sizep3: u8,
        prior: u8,
    ) -> Self {
        Self {
            colors: [
                colbk as u32,
                colpf0 as u32,
                colpf1 as u32,
                colpf2 as u32,
                colpf3 as u32,
                0,
                0,
                0,
            ],
            colors_pm: [colpm0 as u32, colpm1 as u32, colpm2 as u32, colpm3 as u32],
            player_pos: [hposp0 as f32, hposp1 as f32, hposp2 as f32, hposp3 as f32],
            player_size: [
                player_size(sizep0),
                player_size(sizep1),
                player_size(sizep2),
                player_size(sizep3),
            ],
            prior: prior as u32,
            _fill: [0, 0, 0],
        }
    }
}

fn player_size(sizep: u8) -> f32 {
    match sizep & 3 {
        1 => 32.0,
        3 => 64.0,
        _ => 16.0,
    }
}

unsafe impl Byteable for GTIARegsArray {}
impl_render_resource_bytes!(GTIARegsArray);

#[derive(RenderResources, TypeUuid, Debug)]
#[uuid = "1e08866c-0b8a-437e-8bce-37733b25127e"]
pub struct  AnticLine {
    pub line_width: f32,
    pub mode: u32,
    pub hscrol: f32,
    pub line_height: f32,
    pub line_voffset: f32,
    pub data: LineData,
    pub gtia_regs_array: GTIARegsArray,
    pub charset: Charset,
    #[render_resources(ignore)]
    pub start_scan_line: usize,
    #[render_resources(ignore)]
    pub end_scan_line: usize,
}
#[derive(RenderResources, Default, TypeUuid)]
#[uuid = "f145d910-99c5-4df5-b673-e822b1389222"]
pub struct AtariPalette {
    pub palette: Palette,
}
