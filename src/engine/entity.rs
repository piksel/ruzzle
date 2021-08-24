
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

pub struct BgEntity {
    geometry: VertexBuffers<gpu::BgPoint, u16>,
    pub(crate) ranges: [ Range<u32>; 1 ],
    pub(crate) renderer: Option<EntityRenderer>,
}

pub struct EntityRenderer {
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
    // vert_shader: wgpu::ShaderModule,
    // frag_shader: wgpu::ShaderModule,
    pub(crate) render_pipeline: wgpu::RenderPipeline,
}

pub struct GeoEntity {
    pub(crate) scale: f32,
    pub(crate) instances: usize,
    geometry: VertexBuffers<gpu::GpuVertex, u16>,
    pub(crate) ranges: [ Range<u32>; 2 ],
    pub primitives: Vec<Primitive>,
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
}

// fn render_entity(pass: &mut RenderPass, er: &EntityRenderer, rd: &RenderData,
//                      ranges: &[Range<u32>], instances: u32) {
//     pass.set_pipeline(&er.render_pipeline);
//     pass.set_bind_group(0, &rd.bind_group, &[]);
//     pass.set_index_buffer(er.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
//     pass.set_vertex_buffer(0, er.vertex_buffer.slice(..));
//
//     for range in ranges {
//         pass.draw_indexed(range.clone(), 0, 0..instances);
//     }
// }

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
}


pub enum Entity {
    Bg(BgEntity),
    Geo(GeoEntity),
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

impl GeoEntity {

    pub(crate) fn new(
        geometry: VertexBuffers<gpu::GpuVertex, u16>,
        range_fill: Range<u32>,
        range_stroke: Range<u32>, scale: f32,
        instances: usize,
    ) -> Self {




        let prim_count = instances;
        // We cannot send more than PRIM_BUFFER_LEN per bound buffer
        // TODO: Check this earlier?
        assert!(prim_count <= PRIM_BUFFER_LEN);
        let mut primitives = Vec::with_capacity(PRIM_BUFFER_LEN);
        for _ in 0..PRIM_BUFFER_LEN {
            primitives.push(Primitive::new_with_scale(scale) );
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
        let frag = wgpu::include_spirv!(concat!(env!("OUT_DIR"), "./spirv/geometry.frag.spv"));
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


impl BgEntity {
    pub(crate) fn new(geometry: VertexBuffers<gpu::BgPoint, u16>) -> Self {
        BgEntity {
            geometry,
            ranges: [ 0..6 ],
            renderer: None
        }
    }

    pub(crate) fn init_render(&mut self, device: &Device, rd: &RenderData) {
        let geometry = &self.geometry;

        let vert = wgpu::include_spirv!(concat!(env!("OUT_DIR"), "./spirv/background.vert.spv"));
        let vert_shader = device.create_shader_module(&vert);
        let frag = wgpu::include_spirv!(concat!(env!("OUT_DIR"), "./spirv/background.frag.spv"));
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

pub enum BluePrint {
    SvgPath(Box<dyn Fn(&mut dyn SvgPathBuilder)>),
    //Rect(LyM::Rect)
    Square(f32)
}

pub struct EntityBuilder {
    fill_tess: Tes::FillTessellator,
    stroke_tess: Tes::StrokeTessellator,
    fill_opts: FillOptions,
    stroke_opts: Tes::StrokeOptions,
}


const TOLERANCE: f32 = 0.02;


impl EntityBuilder {

    // TODO: BackgroundFillOptions: &FillOptions::DEFAULT

    pub fn new() -> EntityBuilder {
        EntityBuilder{
            fill_tess: FillTessellator::new(),
            stroke_tess: StrokeTessellator::new(),
            fill_opts: Tes::FillOptions::tolerance(TOLERANCE)
                .with_fill_rule(FillRule::NonZero),
            stroke_opts: Tes::StrokeOptions::tolerance(TOLERANCE),
        }
    }

    pub(crate) fn build_bg(&mut self, rect: LyM::Rect) -> Result<BgEntity, TessellationError> {
        let mut geometry = VertexBuffers::new(); //<gpu::BgPoint, u16>

        self.fill_tess.tessellate_rectangle(
            &rect,
            &FillOptions::DEFAULT,
            &mut BuffersBuilder::new(&mut geometry, gpu::Custom)
        )?;

        Ok(BgEntity::new(geometry))
    }

    pub fn tes_fill(&mut self, path: &Path, buf: &mut VertexBuffers<gpu::GpuVertex, u16>, entity_id: u32) -> Result<(), TessellationError> {
        let mut buf_builder = BuffersBuilder::new(
             buf,
            gpu::WithId(entity_id as i32));
        self.fill_tess.tessellate_path(path,&self.fill_opts, &mut buf_builder)?;
        Ok(())
    }

    pub fn tes_stroke(&mut self, path: &Path, buf: &mut VertexBuffers<gpu::GpuVertex, u16>, entity_id: u32) -> Result<(), TessellationError> {
        let mut buf_builder = BuffersBuilder::new(
             buf,
            gpu::WithId(entity_id as i32));
        self.stroke_tess.tessellate_path(path,&self.stroke_opts, &mut buf_builder)?;
        Ok(())
    }

    pub fn build_geo(&mut self, path: &Path, instances: usize, scale: f32, fill: bool, stroke: bool) -> Result<GeoEntity, TessellationError> {
        let mut geometry = VertexBuffers::new();
        let entity_id = 0; // TODO: This needs to be changed for multiple entities in the same buffer
        let fill_id = 0;
        if fill {
            self.tes_fill(path, &mut geometry, entity_id)?;
        }
        let range_fill = fill_id..geometry.indices.len() as u32;
        let stroke_id = range_fill.end;
        if stroke {
            self.tes_stroke(path, &mut geometry, entity_id)?;
        }
        let range_stroke = stroke_id..geometry.indices.len() as u32;
        Ok(GeoEntity::new(geometry, range_fill, range_stroke, scale, instances))
    }

}

// fn path_from_svg_builder<B: SvgPathBuilder + ?Sized, F: Fn(&mut B)>(build_path: F) -> Path {
//     let mut builder = ;
//
// }

