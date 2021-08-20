use lyon::extra::rust_logo::build_logo_path;
use lyon::math::*;
use lyon::path::iterator::PathIterator;
use lyon::path::builder::PathBuilder;
use lyon::path::Path;
use lyon::tessellation;
use lyon::tessellation::geometry_builder::*;
use lyon::tessellation::{FillOptions, FillTessellator};
use lyon::tessellation::{StrokeOptions, StrokeTessellator};

use lyon::algorithms::walk;

use winit::dpi::PhysicalSize;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent, MouseScrollDelta};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

// For create_buffer_init()
use wgpu::util::DeviceExt;

use futures::executor::block_on;
use std::ops::Rem;
use std::time::{Duration, Instant};
use std::error::Error;
use wgpu::SwapChainError;

use ruzzle::paths::{build_tetrion_path};
use ruzzle::tetrominos::{Tetromino, TETRO_COLORS, TetroShape};
use rand::Rng;
use ruzzle::tetrominos;

//use log;



#[repr(C)]
#[derive(Copy, Clone)]
struct Globals {
    resolution: [f32; 2],
    scroll_offset: [f32; 2],
    zoom: f32,
    _pad: f32,
}

unsafe impl bytemuck::Pod for Globals {}
unsafe impl bytemuck::Zeroable for Globals {}

#[repr(C)]
#[derive(Copy, Clone)]
struct GpuVertex {
    position: [f32; 2],
    normal: [f32; 2],
    prim_id: i32,
}
unsafe impl bytemuck::Pod for GpuVertex {}
unsafe impl bytemuck::Zeroable for GpuVertex {}

#[repr(C)]
#[derive(Copy, Clone)]
struct Primitive {
    color: [f32; 4],
    translate: [f32; 2],
    z_index: i32,
    width: f32,
    angle: f32,
    scale: f32,
    _pad1: i32,
    _pad2: i32,
}

impl Primitive {
    const DEFAULT: Self = Primitive {
        color: [0.0; 4],
        translate: [0.0; 2],
        z_index: 0,
        width: 0.0,
        angle: 0.0,
        scale: 1.0,
        _pad1: 0,
        _pad2: 0,
    };
}

unsafe impl bytemuck::Pod for Primitive {}
unsafe impl bytemuck::Zeroable for Primitive {}

#[repr(C)]
#[derive(Copy, Clone)]
struct BgPoint {
    point: [f32; 2],
}
unsafe impl bytemuck::Pod for BgPoint {}
unsafe impl bytemuck::Zeroable for BgPoint {}

const PRIM_BUFFER_LEN: usize = 340;

const DEFAULT_WINDOW_WIDTH: f32 = 1024.0;
const DEFAULT_WINDOW_HEIGHT: f32 = 768.0;

const PLAYFIELD_COLS: u32 = 10;
const PLAYFIELD_ROWS: u32 = 16;
const PLAYFIELD_SIZE: u32 = PLAYFIELD_COLS * PLAYFIELD_ROWS;
const TETRION_SIZE: f32 = 8.0;

/// Creates a texture that uses MSAA and fits a given swap chain
fn create_multisampled_framebuffer(
    device: &wgpu::Device,
    sc_desc: &wgpu::SwapChainDescriptor,
    sample_count: u32,
) -> wgpu::TextureView {
    let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
        label: Some("Multisampled frame descriptor"),
        size: wgpu::Extent3d {
            width: sc_desc.width,
            height: sc_desc.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: sc_desc.format,
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
    };

    device
        .create_texture(multisampled_frame_descriptor)
        .create_view(&wgpu::TextureViewDescriptor::default())
}

fn main() {
    env_logger::init();
    println!("== wgpu example ==");
    println!("Controls:");
    println!("  Arrow keys: scrolling");
    println!("  PgUp/PgDown: zoom in/out");
    println!("  b: toggle drawing the background");
    println!("  a/z: increase/decrease the stroke width");

    // Number of samples for anti-aliasing
    // Set to 1 to disable
    let sample_count = 4;




    let num_instances: usize = PLAYFIELD_SIZE as usize;
    let tolerance = 0.02;

    let fill_prim_id = 0;
    let stroke_prim_id: usize = num_instances;
    //let arrows_prim_id = num_instances + 1;

    let mut geometry: VertexBuffers<GpuVertex, u16> = VertexBuffers::new();

    let mut fill_tess = FillTessellator::new();
    let mut stroke_tess = StrokeTessellator::new();

    // Build a Path for the rust logo.
    let mut builder = Path::builder().with_svg();

    let tetrion_path_scale = 0.8;
    build_tetrion_path(&mut builder);

    // let tetrion_path_scale = 0.06;
    // build_logo_path(&mut builder);

    let tetrion_path = builder.build();

    // Build a Path for the arrow.
    // let mut builder = Path::builder();
    // build_arrow_path(&mut builder);
    // let arrow_path = builder.build();

    fill_tess
        .tessellate_path(
            &tetrion_path,
            &FillOptions::tolerance(tolerance).with_fill_rule(tessellation::FillRule::NonZero),
            &mut BuffersBuilder::new(&mut geometry, WithId(fill_prim_id as i32)),
        )
        .unwrap();

    let fill_range = 0..(geometry.indices.len() as u32);

    stroke_tess
        .tessellate_path(
            &tetrion_path,
            &StrokeOptions::tolerance(tolerance),
            &mut BuffersBuilder::new(&mut geometry, WithId(stroke_prim_id as i32)),
        )
        .unwrap();

    let stroke_range = fill_range.end..(geometry.indices.len() as u32);

    // fill_tess
    //     .tessellate_path(
    //         &arrow_path,
    //         &FillOptions::tolerance(tolerance),
    //         &mut BuffersBuilder::new(&mut geometry, WithId(arrows_prim_id as i32)),
    //     )
    //     .unwrap();
    //
    // let arrow_range = stroke_range.end..(geometry.indices.len() as u32);

    let mut bg_geometry: VertexBuffers<BgPoint, u16> = VertexBuffers::new();

    fill_tess
        .tessellate_rectangle(
            &Rect::new(point(-1.0, -1.0), size(2.0, 2.0)),
            &FillOptions::DEFAULT,
            &mut BuffersBuilder::new(&mut bg_geometry, Custom),
        )
        .unwrap();

    let mut cpu_primitives = Vec::with_capacity(PRIM_BUFFER_LEN);
    for _ in 0..PRIM_BUFFER_LEN {
        cpu_primitives.push(Primitive {
            color: [1.0, 0.0, 0.0, 1.0],
            z_index: 0,
            width: 0.0,
            translate: [0.0, 0.0],
            angle: 0.0,
            ..Primitive::DEFAULT
        });
    }

    let mut playfield = Vec::with_capacity(PLAYFIELD_SIZE as usize);

    // let rng = rand::thread_rng();

    for r in 0..PLAYFIELD_ROWS as usize {
        let rb = r / 4;
        let ri = r % 4;
        for c in 0..PLAYFIELD_COLS as usize {

            let tetind = if c < 5 { rb } else { 4 + rb };
            let ci = if c < 5 { c } else { (PLAYFIELD_COLS as usize - c)-1 };
            // let ci = c % 5;

            let tetromino = &tetrominos::ALL[tetind];

            let tetrion = if match tetromino {
                //_ if c == 5 => false,
                TetroShape::Odd(t) if ci < 3 && ri < 3 => t[ri][ci],
                TetroShape::Even(t) if ci < 4 => t[ri][ci],
                _ => false
            } { tetind } else { 0 };

            // let tetrion = if c < 5 && tetrow[c] { tetind } else { 0 };
            // let tet = match c {
            //     3..=7 => match r {
            //         1..=2 =>
            //         _ => 0
            //     }
            //     _ => 0
            // };
            //let tet = rng.gen_range(0..tetrominos::TL);
            playfield.push(tetrion);
            println!("{:?},{:?} => {:?}", r, c, tetrion)
        }
    }

    // // Stroke primitive
    // cpu_primitives[stroke_prim_id] = Primitive {
    //     color: [0.0, 0.0, 0.0, 1.0],
    //     z_index: num_instances as i32 + 2,
    //     width: 1.0,
    //     ..Primitive::DEFAULT
    // };
    // // Main fill primitive
    // cpu_primitives[fill_prim_id] = Primitive {
    //     color: [1.0, 1.0, 1.0, 1.0],
    //     z_index: num_instances as i32 + 1,
    //     ..Primitive::DEFAULT
    // };
    // Instance primitives
    for (idx, cpu_prim) in cpu_primitives
        .iter_mut()
        .enumerate()
        //.skip(fill_prim_id)
        .take(num_instances * 2)
    {
        let id = (idx % num_instances);
        let tet = playfield[id];
        let color = tetrominos::Colors[playfield[id]];
        let idf = id as f32;
        let col = idf % (PLAYFIELD_COLS as f32);
        let row = (idf / (PLAYFIELD_COLS as f32)).floor();
        cpu_prim.translate = [
            col * TETRION_SIZE,
            row * TETRION_SIZE,
        ];
        cpu_prim.z_index = (id as u32 + 1) as i32;


        if tet == 0 {
            cpu_prim.z_index = 1;
            cpu_prim.color = [0.0, 0.0, 0.0, 1.0];
        } else {
            cpu_prim.color = color;
            // let red = col / (PLAYFIELD_COLS as f32);
            // cpu_prim.color = [
            //     color[0] + red,
            //     color[1] + red,
            //     color[2] + red,
            //     red,
            // ];
        }


        // [
        //     color[0],
        //     // (0.1 * idf).rem(1.0),
        //     //(0.5 * idf).rem(1.0),
        //     //(0.9 * idf).rem(1.0),
        //     color[1],
        //     row / (PLAYFIELD_ROWS as f32),
        //     if idx > num_instances { 1.0 } else { 0.5 },
        // ];
        cpu_prim.scale = tetrion_path_scale;
        if idx >= num_instances {
            cpu_prim.width = 0.3;
            // cpu_prim.z_index = (num_instances as i32) + idx
            cpu_prim.color = [0.0, 0.0, 0.0, 1.0];
        }

        println!("{:?},{:?} RED @ {:?}: {:?}", col, row, color, tetrominos::NAMES[tet]);
    }

    let init_z = 5.0;
    let init_x = ((PLAYFIELD_COLS as f32) * TETRION_SIZE) / 2.0;
    let init_y = ((PLAYFIELD_ROWS as f32) * TETRION_SIZE) / 2.0;

    let mut scene = SceneParams {
        target_zoom: 5.0,
        zoom: init_z,
        target_scroll: vector(init_x, init_y),
        scroll: vector(init_x, init_y),
        show_points: true,
        stroke_width: 1.0,
        target_stroke_width: 1.0,
        draw_background: true,
        cursor_position: (0.0, 0.0),
        window_size: PhysicalSize::new(DEFAULT_WINDOW_WIDTH as u32, DEFAULT_WINDOW_HEIGHT as u32),
        size_changed: true,
    };

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();

    // create an instance
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

    // create an surface
    let surface = unsafe { instance.create_surface(&window) };

    // create an adapter
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: Some(&surface),
    }))
        .unwrap();

    let info = adapter.get_info();
    println!("Backend: {:?}", info.backend);
    println!("Adapter: {} ({:04x}:{:04x})", info.name, info.vendor, info.device);

    // create a device and a queue
    let (device, queue) = block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            features: wgpu::Features::default(),
            limits: wgpu::Limits::default(),
        },
        None,
    ))
        .unwrap();

    let vbo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&geometry.vertices),
        usage: wgpu::BufferUsage::VERTEX,
    });

    let ibo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&geometry.indices),
        usage: wgpu::BufferUsage::INDEX,
    });

    let bg_vbo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&bg_geometry.vertices),
        usage: wgpu::BufferUsage::VERTEX,
    });

    let bg_ibo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&bg_geometry.indices),
        usage: wgpu::BufferUsage::INDEX,
    });

    let prim_buffer_byte_size = (PRIM_BUFFER_LEN * std::mem::size_of::<Primitive>()) as u64;
    let globals_buffer_byte_size = std::mem::size_of::<Globals>() as u64;

    let prims_ubo = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Prims ubo"),
        size: prim_buffer_byte_size,
        usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        mapped_at_creation: false,
    });

    let globals_ubo = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Globals ubo"),
        size: globals_buffer_byte_size,
        usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        mapped_at_creation: false,
    });

    let vs_module =
        &device.create_shader_module(&wgpu::include_spirv!("./../shaders/geometry.vert.spv"));
    let fs_module =
        &device.create_shader_module(&wgpu::include_spirv!("./../shaders/geometry.frag.spv"));
    let bg_vs_module =
        &device.create_shader_module(&wgpu::include_spirv!("./../shaders/background.vert.spv"));
    let bg_fs_module =
        &device.create_shader_module(&wgpu::include_spirv!("./../shaders/background.frag.spv"));

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Bind group layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(globals_buffer_byte_size),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(prim_buffer_byte_size),
                },
                count: None,
            },
        ],
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Bind group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(globals_ubo.as_entire_buffer_binding()),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(prims_ubo.as_entire_buffer_binding()),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
        label: None,
    });

    let depth_stencil_state = Some(wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::Greater,
        stencil: wgpu::StencilState {
            front: wgpu::StencilFaceState::IGNORE,
            back: wgpu::StencilFaceState::IGNORE,
            read_mask: 0,
            write_mask: 0,
        },
        bias: wgpu::DepthBiasState::default(),
    });

    let mut render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &vs_module,
            entry_point: "main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<GpuVertex>() as u64,
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
            module: &fs_module,
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
        depth_stencil: depth_stencil_state.clone(),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
    };

    let render_pipeline = device.create_render_pipeline(&render_pipeline_descriptor);

    // TODO: this isn't what we want: we'd need the equivalent of VK_POLYGON_MODE_LINE,
    // but it doesn't seem to be exposed by wgpu?
    render_pipeline_descriptor.primitive.topology = wgpu::PrimitiveTopology::LineList;

    let bg_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &bg_vs_module,
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
            module: &bg_fs_module,
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
        depth_stencil: depth_stencil_state.clone(),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
    });

    let size = window.inner_size();

    if size != scene.window_size {
        window.set_inner_size(scene.window_size);
    }

    println!("Window inner size: {:?}", size);
    // scene.window_size = size;

    let mut swap_chain_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8Unorm,
        width: scene.window_size.width,
        height: scene.window_size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    let mut multisampled_render_target = None;

    let mut swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);

    let mut depth_texture_view = None;

    let start = Instant::now();
    let mut next_report = start + Duration::from_secs(1);
    let mut frame_count: u32 = 0;
    let mut time_secs: f32 = 0.0;

    event_loop.run(move |event, _, control_flow| {
        if update_inputs(event, control_flow, &mut scene) {
            // keep polling inputs.
            return;
        }

        if scene.size_changed {
            scene.size_changed = false;
            let physical = scene.window_size;
            swap_chain_desc.width = physical.width;
            swap_chain_desc.height = physical.height;
            swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);

            println!("Size changed! New size: {}, {} Updating surface...", physical.width, physical.height);

            let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth texture"),
                size: wgpu::Extent3d {
                    width: swap_chain_desc.width,
                    height: swap_chain_desc.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            });

            depth_texture_view =
                Some(depth_texture.create_view(&wgpu::TextureViewDescriptor::default()));

            multisampled_render_target = if sample_count > 1 {
                Some(create_multisampled_framebuffer(
                    &device,
                    &swap_chain_desc,
                    sample_count,
                ))
            } else {
                None
            };
        }

        let frame = match swap_chain.get_current_frame() {
            Ok(frame) => frame,

            Err(e) => {
                eprintln!("Swap-chain error: {}", e);
                if e == SwapChainError::Outdated {
                    scene.window_size = window.inner_size();
                    scene.size_changed = true;
                }
                return;
            }
        };

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Encoder"),
        });

        // draw_logo(&cpu_primitives, time_secs);

        // cpu_primitives[stroke_prim_id as usize].width = scene.stroke_width;
        // cpu_primitives[stroke_prim_id as usize].color = [
        //     (time_secs * 0.8 - 1.6).sin() * 0.1 + 0.1,
        //     (time_secs * 0.5 - 1.6).sin() * 0.1 + 0.1,
        //     (time_secs - 1.6).sin() * 0.1 + 0.1,
        //     1.0,
        // ];

        for idx in 0..num_instances {
            // let mut cpu_prim = cpu_primitives[(fill_prim_id + idx) as usize];
            // cpu_prim.width = scene.stroke_width;
            cpu_primitives[(fill_prim_id + idx) as usize].translate = [
                ((time_secs * 0.05 * idx as f32).sin() * 1.0) + ((idx % PLAYFIELD_COLS as usize) as f32 * TETRION_SIZE),
                //(time_secs * 0.05 * idx as f32).sin() * (100.0 + idx as f32 * 10.0),
                ((time_secs * 0.1 * idx as f32).sin() * 1.0) + ((idx / PLAYFIELD_COLS as usize) as f32 * TETRION_SIZE)
                //(time_secs * 0.1 * idx as f32).sin() * (100.0 + idx as f32 * 10.0),
            ];

            cpu_primitives[(stroke_prim_id + idx) as usize].translate = [
                ((time_secs * 0.05 * idx as f32).sin() * 1.0) + ((idx % PLAYFIELD_COLS as usize) as f32 * TETRION_SIZE),
                //(time_secs * 0.05 * idx as f32).sin() * (100.0 + idx as f32 * 10.0),
                ((time_secs * 0.1 * idx as f32).sin() * 1.0) + ((idx / PLAYFIELD_COLS as usize) as f32 * TETRION_SIZE)
                //(time_secs * 0.1 * idx as f32).sin() * (100.0 + idx as f32 * 10.0),
            ];
        }

        // let mut arrow_count = 0;
        // let offset = (time_secs * 10.0).rem(5.0);
        // walk::walk_along_path(
        //     logo_path.iter().flattened(0.01),
        //     offset,
        //     &mut walk::RepeatedPattern {
        //         callback: |pos: Point, tangent: Vector, _| {
        //             if arrow_count + num_instances as usize + 1 >= PRIM_BUFFER_LEN {
        //                 // Don't want to overflow the primitive buffer,
        //                 // just skip the remaining arrows.
        //                 return false;
        //             }
        //             cpu_primitives[arrows_prim_id as usize + arrow_count] = Primitive {
        //                 color: [0.7, 0.9, 0.8, 1.0],
        //                 translate: (pos * 2.3 - vector(80.0, 80.0)).to_array(),
        //                 angle: tangent.angle_from_x_axis().get(),
        //                 scale: 2.0,
        //                 z_index: arrows_prim_id as i32,
        //                 ..Primitive::DEFAULT
        //             };
        //             arrow_count += 1;
        //             true
        //         },
        //         intervals: &[5.0, 5.0, 5.0],
        //         index: 0,
        //     },
        // );

        queue.write_buffer(
            &globals_ubo,
            0,
            bytemuck::cast_slice(&[Globals {
                resolution: [
                    scene.window_size.width as f32,
                    scene.window_size.height as f32,
                ],
                zoom: scene.zoom,
                scroll_offset: scene.scroll.to_array(),
                _pad: 0.0,
            }]),
        );

        queue.write_buffer(&prims_ubo, 0, bytemuck::cast_slice(&cpu_primitives));

        {
            // A resolve target is only supported if the attachment actually uses anti-aliasing
            // So if sample_count == 1 then we must render directly to the swapchain's buffer
            let color_attachment = if let Some(msaa_target) = &multisampled_render_target {
                wgpu::RenderPassColorAttachment {
                    view: msaa_target,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                    resolve_target: Some(&frame.output.view),
                }
            } else {
                wgpu::RenderPassColorAttachment {
                    view: &frame.output.view,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                    resolve_target: None,
                }
            };

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[color_attachment],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_texture_view.as_ref().unwrap(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: true,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: true,
                    }),
                }),
            });

            pass.set_pipeline(&render_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_index_buffer(ibo.slice(..), wgpu::IndexFormat::Uint16);
            pass.set_vertex_buffer(0, vbo.slice(..));

            pass.draw_indexed(fill_range.clone(), 0, 0..(num_instances as u32));
            pass.draw_indexed(stroke_range.clone(), 0, 0..(num_instances as u32));
            // pass.draw_indexed(arrow_range.clone(), 0, 0..(arrow_count as u32));

            if scene.draw_background {
                pass.set_pipeline(&bg_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_index_buffer(bg_ibo.slice(..), wgpu::IndexFormat::Uint16);
                pass.set_vertex_buffer(0, bg_vbo.slice(..));

                pass.draw_indexed(0..6, 0, 0..1);
            }
        }

        queue.submit(Some(encoder.finish()));

        frame_count += 1;
        let now = Instant::now();
        time_secs = (now - start).as_secs_f32();
        if now >= next_report {
            // println!("{} FPS", frame_count);
            window.set_title(&*format!("{} FPS", frame_count));
            frame_count = 0;
            next_report = now + Duration::from_secs(1);
        }
    });
}




/// This vertex constructor forwards the positions and normals provided by the
/// tessellators and add a shape id.
pub struct WithId(pub i32);

impl FillVertexConstructor<GpuVertex> for WithId {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> GpuVertex {
        GpuVertex {
            position: vertex.position().to_array(),
            normal: [0.0, 0.0],
            prim_id: self.0,
        }
    }
}

impl StrokeVertexConstructor<GpuVertex> for WithId {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex) -> GpuVertex {
        GpuVertex {
            position: vertex.position_on_path().to_array(),
            normal: vertex.normal().to_array(),
            prim_id: self.0,
        }
    }
}

pub struct Custom;

impl FillVertexConstructor<BgPoint> for Custom {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> BgPoint {
        BgPoint {
            point: vertex.position().to_array(),
        }
    }
}

struct SceneParams {
    target_zoom: f32,
    zoom: f32,
    target_scroll: Vector,
    scroll: Vector,
    show_points: bool,
    stroke_width: f32,
    target_stroke_width: f32,
    draw_background: bool,
    cursor_position: (f32, f32),
    window_size: PhysicalSize<u32>,
    size_changed: bool,
}

fn update_inputs(
    event: Event<()>,
    control_flow: &mut ControlFlow,
    scene: &mut SceneParams,
) -> bool {
    match event {
        Event::MainEventsCleared => {
            return false;
        }
        Event::WindowEvent {
            event: WindowEvent::Destroyed,
            ..
        }
        | Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
            return false;
        }
        Event::WindowEvent {
            event: WindowEvent::MouseWheel { delta, .. },
            ..
        } => {
            let ydelta = match delta {
                MouseScrollDelta::LineDelta( x, y ) => y,
                MouseScrollDelta::PixelDelta( pp) => pp.y as f32
            };
            scene.target_zoom *= if ydelta < 0.0 { 0.8 } else { 1.25 } ;
        }
        Event::WindowEvent {
            event: WindowEvent::CursorMoved { position, .. },
            ..
        } => {
            scene.cursor_position = (position.x as f32, position.y as f32);
        }
        Event::WindowEvent {
            event: WindowEvent::Resized(size),
            ..
        } => {
            scene.window_size = size;
            scene.size_changed = true
        }
        Event::WindowEvent {
            event:
            WindowEvent::KeyboardInput {
                input:
                KeyboardInput {
                    state: ElementState::Pressed,
                    virtual_keycode: Some(key),
                    ..
                },
                ..
            },
            ..
        } => match key {
            VirtualKeyCode::Escape => {
                *control_flow = ControlFlow::Exit;
                return false;
            }
            VirtualKeyCode::PageDown => {
                scene.target_zoom *= 0.8;
            }
            VirtualKeyCode::PageUp => {
                scene.target_zoom *= 1.25;
            }
            VirtualKeyCode::Left => {
                scene.target_scroll.x -= 50.0 / scene.target_zoom;
            }
            VirtualKeyCode::Right => {
                scene.target_scroll.x += 50.0 / scene.target_zoom;
            }
            VirtualKeyCode::Up => {
                scene.target_scroll.y -= 50.0 / scene.target_zoom;
            }
            VirtualKeyCode::Down => {
                scene.target_scroll.y += 50.0 / scene.target_zoom;
            }
            VirtualKeyCode::P => {
                scene.show_points = !scene.show_points;
            }
            VirtualKeyCode::B => {
                scene.draw_background = !scene.draw_background;
            }
            VirtualKeyCode::A => {
                scene.target_stroke_width /= 0.8;
            }
            VirtualKeyCode::Z => {
                scene.target_stroke_width *= 0.8;
            }
            _key => {}
        },
        _evt => {
            //println!("{:?}", _evt);
        }
    }
    //println!(" -- zoom: {}, scroll: {:?}", scene.target_zoom, scene.target_scroll);

    scene.zoom += (scene.target_zoom - scene.zoom) / 3.0;
    scene.scroll = scene.scroll + (scene.target_scroll - scene.scroll) / 3.0;
    scene.stroke_width =
        scene.stroke_width + (scene.target_stroke_width - scene.stroke_width) / 5.0;

    *control_flow = ControlFlow::Poll;

    true
}