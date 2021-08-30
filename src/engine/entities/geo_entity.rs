use crate::engine::{gpu, RenderData};
use crate::engine::entities::{EntityRenderer, init_device_buffers};
use lyon::tessellation::{FillTessellator, StrokeTessellator, VertexBuffers, FillRule, FillOptions, BuffersBuilder, TessellationError};
use std::ops::Range;

pub struct GeoEntity {
    pub(crate) scale: f32,
    pub(crate) instances: usize,
    geometry: VertexBuffers<gpu::GpuVertex, u16>,
    pub(crate) ranges: [ Range<u32>; 2 ],
    pub primitives: Vec<gpu::Primitive>,
    pub(crate) renderer: Option<EntityRenderer>,
}

impl GeoEntity {
    //pub(crate) fn render<'a>(&'a self, pass: &'a mut wgpu::RenderPass<'a>, rd: &RenderData) {
    // let cpu_primitives = &self.primitives;
    // let prims_ubo = &self.prims_buffer;



    // render_entity(pass,
    //             &self.renderer.unwrap(),
    //             rd,
    //             &self.ranges,
    //             self.instances as u32,
    // );

    // pass.set_pipeline(&geo_entity.render_pipeline);
    // pass.set_bind_group(0, &geo_entity.bind_group, &[]);
    // pass.set_index_buffer(geo_entity.index_buffer.slice(..),
    //                       wgpu::IndexFormat::Uint16);
    // pass.set_vertex_buffer(0, geo_entity.vertex_buffer.slice(..));
    //
    // let num_instances = geo_entity.range_fill.len();
    // pass.draw_indexed(geo_entity.range_fill.clone(), 0, 0..(num_instances as u32));
    // pass.draw_indexed(geo_entity.range_stroke.clone(), 0, 0..(num_instances as u32));
    // pass.draw_indexed(arrow_range.clone(), 0, 0..(arrow_count as u32));

    //}

    pub(crate) fn new(
        geometry: VertexBuffers<gpu::GpuVertex, u16>,
        range_fill: Range<u32>,
        range_stroke: Range<u32>, scale: f32,
        instances: usize,
    ) -> Self {


        let prim_count = instances;
        // We cannot send more than PRIM_BUFFER_LEN per bound buffer
        // TODO: Check this earlier?
        assert!(prim_count <= gpu::PRIM_BUFFER_LEN);
        let mut primitives = Vec::with_capacity(gpu::PRIM_BUFFER_LEN);
        for _ in 0..gpu::PRIM_BUFFER_LEN {
            primitives.push(gpu::Primitive::new_with_scale(scale) );
        }


        GeoEntity {
            scale,
            geometry,
            ranges: [ range_fill, range_stroke ],
            instances,
            primitives,
            renderer: None,
        }
    }

    pub(crate) fn init_render(&mut self, device: &wgpu::Device, rd: &RenderData) {

        let (vertex_buffer, index_buffer) = init_device_buffers(&self.geometry, device);


        let vert = wgpu::include_spirv!(concat!(env!("OUT_DIR"), "/spirv/geometry.vert.spv"));
        let vert_shader = device.create_shader_module(&vert);
        let frag = wgpu::include_spirv!(concat!(env!("OUT_DIR"), "/spirv/geometry.frag.spv"));
        let frag_shader = device.create_shader_module(&frag);

        let mut render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&rd.pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vert_shader,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<gpu::GpuVertex>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            format: wgpu::VertexFormat::Float32x2,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            offset: 8,
                            format: wgpu::VertexFormat::Float32x2,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            offset: 16,
                            format: wgpu::VertexFormat::Sint32,
                            shader_location: 2,
                        },
                    ],
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
        };

        let render_pipeline = device.create_render_pipeline(&render_pipeline_descriptor);

        // TODO: this isn't what we want: we'd need the equivalent of VK_POLYGON_MODE_LINE,
        // but it doesn't seem to be exposed by wgpu?
        render_pipeline_descriptor.primitive.topology = wgpu::PrimitiveTopology::LineList;

        self.renderer = Some(EntityRenderer{
            vertex_buffer,
            index_buffer,
            // vert_shader,
            // frag_shader,
            render_pipeline,
        })
    }

}