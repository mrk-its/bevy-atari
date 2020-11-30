use std::convert::TryInto;
use bevy::asset::Handle;
use bevy::core::{Byteable, Bytes};
use bevy::prelude::Color;
use bevy::render::{
    impl_render_resource_bytes,
    renderer::{RenderResource, RenderResourceType},
    texture::Texture,
};
use crate::gtia::atari_color;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Charset {
    pub data: [u8; 1024],
}

impl Charset {
    pub fn new(src: &[u8]) -> Self {
        Self { data: src.try_into().expect("byte slice of length 1024") }
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
    pub fn new(src: &[u8]) -> Self {
        Self { data: src.try_into().expect("byte slice of length 48") }
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
        Self { data: palette.as_slice().try_into().expect("byte slice of length 256") }
    }
}

unsafe impl Byteable for Palette {}
impl_render_resource_bytes!(Palette);

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct GTIAColors {
    pub colbk: Color,
    pub colpf0: Color,
    pub colpf1: Color,
    pub colpf2: Color,
    pub colpf3: Color,
}
unsafe impl Byteable for GTIAColors {}
impl_render_resource_bytes!(GTIAColors);
