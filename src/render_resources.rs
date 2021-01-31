use crate::system::AtariSystem;
use crate::{antic::ModeLineDescr, gtia::atari_color};
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
use bevy::{
    math::vec2,
    prelude::*,
    render::{mesh::Indices, pipeline::PrimitiveTopology},
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
pub struct VideoMemory {
    pub data: Vec<u8>,
}

impl VideoMemory {
    pub fn push(&mut self, system: &mut AtariSystem, atari_offs: usize, size: usize) -> usize {
        let offset = self.data.len();
        unsafe { self.data.set_len(offset + size) }
        system.antic_copy_to_slice(atari_offs as u16, &mut self.data[offset..offset + size]);
        offset
    }
}

impl VideoMemory {
    fn new(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }
}

impl Bytes for VideoMemory {
    fn write_bytes(&self, buffer: &mut [u8]) {
        self.data.write_bytes(buffer);
    }

    fn byte_len(&self) -> usize {
        self.data.len()
    }

    fn byte_capacity(&self) -> usize {
        self.data.capacity()
    }
}
impl_render_resource_bytes!(VideoMemory);

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
#[derive(Clone, Debug)]
pub struct GTIARegsArray {
    pub regs: Vec<GTIARegs>,
}

impl GTIARegsArray {
    pub fn new(capacity: usize) -> Self {
        let regs = Vec::with_capacity(capacity);
        Self { regs }
    }
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
        self.regs.write_bytes(buffer);
    }

    fn byte_len(&self) -> usize {
        std::mem::size_of::<GTIARegs>() * self.regs.len()
    }

    fn byte_capacity(&self) -> usize {
        std::mem::size_of::<GTIARegs>() * self.regs.capacity()
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
#[uuid = "bea612c2-68ed-4432-8d9c-f03ebea97043"]
pub struct AnticData {
    pub gtia_regs: GTIARegsArray,
    pub video_memory: VideoMemory,
    pub charset_memory: VideoMemory,
    #[render_resources(ignore)]
    pub positions: Vec<[f32; 3]>,
    #[render_resources(ignore)]
    pub custom: Vec<[f32; 4]>,
    #[render_resources(ignore)]
    pub uvs: Vec<[f32; 2]>,
    #[render_resources(ignore)]
    pub indices: Vec<u32>,
}

impl AnticData {
    pub fn clear(&mut self) {
        self.positions.clear();
        self.custom.clear();
        self.uvs.clear();
        self.indices.clear();
        self.video_memory.data.clear();
        self.charset_memory.data.clear();
    }

    pub fn create_mesh(&self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, self.positions.clone());
        mesh.set_attribute("Vertex_Custom", self.custom.clone());
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs.clone());
        mesh.set_indices(Some(Indices::U32(self.indices.clone())));
        mesh
    }

    pub fn create_mode_line(&mut self, mode_line: &ModeLineDescr) {
        let index_offset = self.positions.len() as u32;

        let scan_line_y = mode_line.scan_line as f32 - 8.0;

        let north_west = vec2(-192.0, 120.0 - scan_line_y);
        let north_east = vec2(192.0, 120.0 - scan_line_y);
        let south_west = vec2(-192.0, 120.0 - (scan_line_y + mode_line.height as f32));
        let south_east = vec2(192.0, 120.0 - (scan_line_y + mode_line.height as f32));

        self.positions.push([south_west.x, south_west.y, 0.0]);
        self.positions.push([north_west.x, north_west.y, 0.0]);
        self.positions.push([north_east.x, north_east.y, 0.0]);
        self.positions.push([south_east.x, south_east.y, 0.0]);

        self.uvs.push([0.0, 1.0]);
        self.uvs.push([0.0, 0.0]);
        self.uvs.push([1.0, 0.0]);
        self.uvs.push([1.0, 1.0]);

        let scan_line = mode_line.scan_line as u32 - 8;
        let height = mode_line.height as u32;
        let width = mode_line.width as u32 / 2;

        let b0 = (mode_line.mode as u32 | (scan_line << 8) | (height << 16)) as f32;
        let b1 = (mode_line.hscrol as u32 | ((mode_line.line_voffset as u32) << 8) | (width << 16)) as f32;
        let b2 = mode_line.video_memory_offset as f32;
        let b3 = mode_line.charset_memory_offset as f32;

        self.custom.push([b0, b1, b2, b3]);
        self.custom.push([b0, b1, b2, b3]);
        self.custom.push([b0, b1, b2, b3]);
        self.custom.push([b0, b1, b2, b3]);

        self.indices.extend(
            [
                index_offset + 0,
                index_offset + 2,
                index_offset + 1,
                index_offset + 0,
                index_offset + 3,
                index_offset + 2,
            ]
            .iter(),
        );
    }
}

impl Default for AnticData {
    fn default() -> Self {
        bevy::utils::tracing::info!("creating new AnticData!");
        let mut gtia_regs = GTIARegsArray::new(240);
        unsafe {
            gtia_regs.regs.set_len(240);
        }
        let video_memory = VideoMemory::new(240 * 48);
        // max 30 lines of text mode so:
        let charset_memory = VideoMemory::new(30 * 1024);
        Self {
            gtia_regs,
            video_memory,
            charset_memory,
            positions: Default::default(),
            custom: Default::default(),
            uvs: Default::default(),
            indices: Default::default(),
        }
    }
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
