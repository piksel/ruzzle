pub(crate) mod entity_builder;
pub(crate) mod geo_entity;
pub(crate) mod bg_entity;

pub type BgEntity = crate::engine::entities::bg_entity::BgEntity;
pub type GeoEntity = crate::engine::entities::geo_entity::GeoEntity;
pub type BluePrint = crate::engine::entities::entity_builder::BluePrint;
pub type EntityBuilder = crate::engine::entities::entity_builder::EntityBuilder;

use lyon::tessellation as Tes;
use lyon::path::math as LyM;
use lyon::tessellation::{FillTessellator, StrokeTessellator, VertexBuffers, FillRule, FillOptions, BuffersBuilder, TessellationError};
use lyon::path::{Path};
use lyon::path::builder::{SvgPathBuilder};
use std::ops::Range;
use crate::engine::{gpu, RenderData};
use crate::engine::gpu::{Primitive, PRIM_BUFFER_LEN};
use wgpu::{Device};
use wgpu::util::DeviceExt;
use lyon::math::Point;
use bytemuck::Pod;

pub enum Entity {
    Bg(BgEntity),
    Geo(GeoEntity),
}

pub struct EntityToken<'a> {
    index: usize,
    stroke_range: Range<u32>,
    fill_range: Range<u32>,
    pub label: Option<&'a str>,
}

impl<'a> EntityToken<'a> {
    pub(crate) fn new(index: usize, entity: &GeoEntity) -> Self {
        let stroke_range = entity.ranges[0].clone();
        let fill_range = entity.ranges[1].clone();
        EntityToken{
            index,
            stroke_range,
            fill_range,
            label: None,
        }
    }
}

pub struct EntityRenderer {
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
    // vert_shader: wgpu::ShaderModule,
    // frag_shader: wgpu::ShaderModule,
    pub(crate) render_pipeline: wgpu::RenderPipeline,
}

fn init_device_buffers<V>(vbuffer: &VertexBuffers<V, u16>, device: &Device)
    -> (wgpu::Buffer, wgpu::Buffer) where V: Pod {
    (device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&vbuffer.vertices),
        usage: wgpu::BufferUsage::VERTEX,
    }),
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&vbuffer.indices),
        usage: wgpu::BufferUsage::INDEX,
    }))
}






// fn path_from_svg_builder<B: SvgPathBuilder + ?Sized, F: Fn(&mut B)>(build_path: F) -> Path {
//     let mut builder = ;
//
// }

