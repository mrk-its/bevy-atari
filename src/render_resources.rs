use crate::gtia::atari_color;
use bevy::asset::Handle;
use bevy::core::{Byteable, Bytes};
use bevy::prelude::Color;
use bevy::reflect::TypeUuid;
use bevy::render::renderer::RenderResources;
use bevy::render::{
    impl_render_resource_bytes,
    renderer::{RenderResource, RenderResourceType},
    texture::Texture,
};
use std::convert::TryInto;
use crate::system::AtariSystem;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Charset {
    pub data: [u8; 1024],
}

impl Default for Charset {
    fn default() -> Self {
        Self { data: [0; 1024] }
    }
}

impl Charset {
    pub fn new(system: &mut AtariSystem, offs: usize) -> Self {
        let mut charset = Charset::default();
        system.copy_to_slice(offs as u16, &mut charset.data);
        charset
    }
}

unsafe impl Byteable for Charset {}
impl_render_resource_bytes!(Charset);

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct LineData {
    pub data: [u8; 48],
}

impl LineData {
    pub fn new(system: &mut AtariSystem, offs: usize) -> Self {
        let mut data = LineData::default(); // TODO - perf
        system.copy_to_slice(offs as u16, &mut data.data);
        data
    }
}

impl Default for LineData {
    fn default() -> Self {
        Self { data: [0; 48] }
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
    pub hposp: [u32; 4],
    pub hposm: [u32; 4],
    pub player_size: [u32; 4],
    pub grafp: [u32; 4],
    pub prior: u32,
    pub sizem: u32,
    pub grafm: u32,
    pub _fill: u32,
}

unsafe impl Byteable for GTIARegsArray {}
impl_render_resource_bytes!(GTIARegsArray);

#[derive(RenderResources, TypeUuid, Debug)]
#[uuid = "1e08866c-0b8a-437e-8bce-37733b25127e"]
pub struct AnticLine {
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
#[derive(RenderResources, Default, TypeUuid, Debug)]
#[uuid = "f145d910-99c5-4df5-b673-e822b1389222"]
pub struct AtariPalette {
    pub palette: Palette,
}
