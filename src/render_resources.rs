use crate::gtia::atari_color;
use crate::system::AtariSystem;
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

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Charset {
    pub data: Vec<u8>,
}

impl Default for Charset {
    fn default() -> Self {
        Self {
            data: Vec::with_capacity(1024),
        }
    }
}

impl Charset {
    pub fn set_data(&mut self, system: &mut AtariSystem, offs: usize, size: usize) {
        unsafe {
            self.data.set_len(size);
        }
        system.antic_copy_to_slice(offs as u16, &mut self.data[..size]);
    }
}

impl Bytes for Charset {
    fn write_bytes(&self, buffer: &mut [u8]) {
        self.data.write_bytes(buffer);
    }

    fn byte_len(&self) -> usize {
        self.data.len()
    }

    fn byte_capacity(&self) -> usize {
        1024
    }
}

impl_render_resource_bytes!(Charset);

#[repr(C)]
#[derive(Clone, Debug)]
pub struct LineData {
    pub data: Vec<u8>,
}

impl LineData {
    pub fn set_data(&mut self, system: &mut AtariSystem, offs: usize, size: usize) {
        if size > 0 {
            unsafe { self.data.set_len(size) }
            system.antic_copy_to_slice(offs as u16, &mut self.data[..size]);
        }
    }
}

impl Default for LineData {
    fn default() -> Self {
        LineData {
            data: Vec::with_capacity(48),
        }
    }
}

impl Bytes for LineData {
    fn write_bytes(&self, buffer: &mut [u8]) {
        self.data.write_bytes(buffer);
    }

    fn byte_len(&self) -> usize {
        self.data.len()
    }

    fn byte_capacity(&self) -> usize {
        48
    }
}
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
#[derive(Default, Clone, Debug)]
pub struct GTIARegsArray {
    pub regs: Vec<GTIARegs>,
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

unsafe impl Byteable for GTIARegs {}
impl_render_resource_bytes!(GTIARegs);

impl Bytes for GTIARegsArray {
    fn write_bytes(&self, buffer: &mut [u8]) {
        assert!(self.regs.len() <= 8);
        self.regs.write_bytes(buffer);
    }

    fn byte_len(&self) -> usize {
        std::mem::size_of::<GTIARegs>() * self.regs.len()
    }

    fn byte_capacity(&self) -> usize {
        std::mem::size_of::<GTIARegs>() * 8
    }
}
impl_render_resource_bytes!(GTIARegsArray);

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct AnticLineDescr {
    pub line_width: f32,
    pub mode: u32,
    pub hscrol: f32,
    pub line_height: f32,
    pub line_voffset: f32,
    pub scan_line: f32,
}

unsafe impl Byteable for AnticLineDescr {}
impl_render_resource_bytes!(AnticLineDescr);

#[derive(RenderResources, TypeUuid, Debug)]
#[uuid = "1e08866c-0b8a-437e-8bce-37733b25127e"]
pub struct AnticLine {
    pub antic_line_descr: AnticLineDescr,
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

#[derive(Debug, RenderResources, TypeUuid)]
#[uuid = "dace545e-4bc6-4595-a79d-1124fa694977"]
pub struct CustomTexture {
    pub color: Color,
    pub texture: Option<Handle<Texture>>,
}
