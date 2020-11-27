use bevy::asset::Handle;
use bevy::core::{Byteable, Bytes};
use bevy::prelude::Color;
use bevy::render::{
    impl_render_resource_bytes,
    renderer::{RenderResource, RenderResourceType},
    texture::Texture,
};

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct ColorSet {
    pub c0: Color,
    pub c1: Color,

    pub c0_0: Color,
    pub c1_0: Color,
    pub c2_0: Color,
    pub c3_0: Color,

    pub c0_1: Color,
    pub c1_1: Color,
    pub c2_1: Color,
    pub c3_1: Color,
}
unsafe impl Byteable for ColorSet {}
impl_render_resource_bytes!(ColorSet);
