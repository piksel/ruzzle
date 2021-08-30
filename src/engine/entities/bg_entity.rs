use crate::engine::{gpu, RenderData};
use crate::engine::entities::{EntityRenderer, init_device_buffers};
use lyon::tessellation::{FillTessellator, StrokeTessellator, VertexBuffers, FillRule, FillOptions, BuffersBuilder, TessellationError};
use std::ops::Range;
use wgpu;
use lyon::math::Point;

pub struct BgEntity {
    geometry: VertexBuffers<gpu::BgPoint, u16>,
    pub(crate) ranges: [ Range<u32>; 1 ],
    pub(crate) renderer: Option<EntityRenderer>,
}

impl BgEntity {
    // pub(crate) fn render<'a>(&'a self, pass: &'a mut wgpu::RenderPass<'a>, rd: &RenderData) {
    //     let draw_ranges: [Range<u32>; 1 ] = [ 0..6 ];
    //     render_entity(pass,
    //                   &self.renderer.unwrap(),
    //                   rd,
    //                     &draw_ranges[..],
    //                     1,
    //     );
    // }

    pub(crate) fn new(geometry: VertexBuffers<gpu::BgPoint, u16>) -> Self {
        BgEntity {
            geometry,
            ranges: [ 0..6 ],
            renderer: None
        }
    }

    pub(crate) fn init_render(&mut self, device: &wgpu::Device, rd: &RenderData) {
        let geometry = &self.geometry;

        let vert = wgpu::include_spirv!(concat!(env!("OUT_DIR"), "/spirv/background.vert.spv"));
        let vert_shader = device.create_shader_module(&vert);
        let frag = wgpu::include_spirv!(concat!(env!("OUT_DIR"), "/spirv/background.frag.spv"));
        let frag_shader = device.create_shader_module(&frag);

        let (vertex_buffer, index_buffer) = init_device_buffers(&geometry, device);

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&rd.pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vert_shader,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Point>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        format: wgpu::VertexFormat::Float32x2,
                        shader_location: 0,
                    }],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &frag_shader,
                entry_point: "main",
                targets: &[
                    wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Bgra8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrite::ALL,
                    },
                ],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                polygon_mode: wgpu::PolygonMode::Fill,
                front_face: wgpu::FrontFace::Ccw,
                strip_index_format: None,
                cull_mode: None,
                clamp_depth: false,
                conservative: false,
            },
            depth_stencil: rd.depth_stencil_state.clone(),
            multisample: wgpu::MultisampleState {
                count: rd.sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });

        self.renderer = Some(EntityRenderer{
            vertex_buffer,
            index_buffer,
            // vert_shader,
            // frag_shader,
            render_pipeline,
        })
    }
}