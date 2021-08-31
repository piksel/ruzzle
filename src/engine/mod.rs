
pub mod error;
pub(crate) mod entities;
pub mod gpu;

const DEFAULT_WINDOW_WIDTH: f32 = 1024.0;
const DEFAULT_WINDOW_HEIGHT: f32 = 768.0;

const PLAYFIELD_COLS: u32 = 10;
const PLAYFIELD_ROWS: u32 = 16;
const PLAYFIELD_SIZE: u32 = PLAYFIELD_COLS * PLAYFIELD_ROWS;
const TETRION_SIZE: f32 = 8.0;

pub use entities::{BluePrint, GeoEntity, Entity, EntityToken};
use lyon::math::{vector, size, point, Vector, Rect};
use winit::event_loop as ELoop;
use winit::window::Window;
use futures::executor::block_on;
use crate::engine::gpu::{Globals, Primitive, PRIM_BUFFER_LEN};
use wgpu::{Device, SwapChainFrame};
use winit::dpi::PhysicalSize;
use crate::engine::entities::BgEntity;
use lyon::path::traits::SvgPathBuilder;
use lyon::path::Path;
use std::time::{Duration, Instant};
use winit::event::{VirtualKeyCode, Event, WindowEvent};
use crate::engine::error::EngineError;
use crate::tetrominos::TetroShape;
use crate::tetrominos;
use rand::rngs::ThreadRng;
use rand::Rng;

use log::{info, warn, error, debug};

pub type EngineResult<T> = Result<T, EngineError>;

pub struct Engine {
    next_report: Instant,
    frame_count: u32,
    anim_start: Instant,
    anim_secs: f32,
    pub geo_entities: Vec<GeoEntity>,
    bg_entities: Vec<BgEntity>,
    scene: SceneParams,
    pub device: wgpu::Device,
    sample_count: u32,
    tolerance: f32,
    
    pub event_loop: Option<ELoop::EventLoop<()>>,
    swap_chain_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,
    depth_texture_view: Option<wgpu::TextureView>,
    multisampled_render_target: Option<wgpu::TextureView>,
    queue: wgpu::Queue,
    pub surface: wgpu::Surface,
    pub window: Window,
    render_data: Option<RenderData>,

    pub playfield: [usize; PLAYFIELD_SIZE as usize],
    pub rng: ThreadRng,
}

pub(crate) struct RenderData {
    globals_buffer: wgpu::Buffer,
    prim_buffers: Vec<wgpu::Buffer>,
    pipeline_layout: wgpu::PipelineLayout,
    bind_group: wgpu::BindGroup,
    depth_stencil_state: Option<wgpu::DepthStencilState>,
    sample_count: u32,
}

fn test(s: impl SvgPathBuilder) {
    drop(s)
}

impl Engine {
    pub fn new(sample_count: u32, tolerance: f32, use_low_power_gpu: bool) -> Self {
        info!("Initializing engine...");

        let init_z = 5.0;
        let init_x = ((PLAYFIELD_COLS as f32) * TETRION_SIZE) / 2.0;
        let init_y = ((PLAYFIELD_ROWS as f32) * TETRION_SIZE) / 2.0;

        let event_loop = ELoop::EventLoop::new();

        let window = Window::new(&event_loop).unwrap();

        // create an instance
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

        // create an surface
        let surface = unsafe { instance.create_surface(&window) };

        let power_preference = if use_low_power_gpu {
            wgpu::PowerPreference::LowPower
        } else {
            wgpu::PowerPreference::HighPerformance
        };

        // create an adapter
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference,
            compatible_surface: Some(&surface),
        }))
            .unwrap();

        let info = adapter.get_info();
        info!("Backend: {:?}", info.backend);
        info!("Adapter: {} ({:04x}:{:04x})", info.name, info.vendor, info.device);

        let scene = SceneParams{
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
            target_piece_x: 0,
            target_piece_y: 0,
            target_tet_rot: 0,
            curr_tet_index: 0,
            curr_tet_pos: [0, 0],
            curr_tet_active: false,
            curr_tet_rot: 0,
            curr_tet_matrix: TetroShape::Odd(tetrominos::TETRO_NONE),
            speed: 1.0,
            last_down_secs: 0.0,
        };

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

        let mut swap_chain_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
            width: scene.window_size.width,
            height: scene.window_size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        let mut swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);

        if window.inner_size() != scene.window_size {
            window.set_inner_size(scene.window_size);
        }


        let anim_start = Instant::now();

        let engine = Engine{
            next_report: anim_start + Duration::from_secs(1),
            frame_count: 0,
            anim_secs: 0.0,
            anim_start,
            bg_entities: Vec::new(),
            geo_entities: Vec::new(),
            scene,
            device,
            sample_count,
            tolerance,
            event_loop: Some(event_loop),
            swap_chain,
            swap_chain_desc,
            depth_texture_view: None,
            multisampled_render_target: None,
            queue,
            surface,
            window,
            render_data: None,
            playfield: [ 0 ; PLAYFIELD_SIZE as usize],
            rng: rand::thread_rng()
        };
        engine
    }

    pub fn create_bg_entity(&mut self, dim: f32) -> Result<(), String> {
        let mut eb = entities::EntityBuilder::new();
        let rect = Rect::new(
            point(-(dim / 2.0), -(dim / 2.0)),
            size(dim, dim));
        match eb.build_bg(rect) {
            Ok(entity) => {
                self.bg_entities.push(entity);
                // let index = self.bg_entities.len()-1;
                Ok(())
            }
            Err(e) => Err(format!("{:?}", e)),
        }
    }



    // pub fn create_geo_entity<B>(&mut self, build_path: fn(&mut B), num_instances: usize) -> impl Entity where B: SvgPathBuilder {
    //     let mut builder = Path::builder().with_svg();
    //     test(builder);
    //     build_path(&mut builder);
    //     let path = builder.build();
    //     let e = entity::EntityBuilder::new().build_geo(&path, num_instances);
    //     return self.add_entity(e)
    // }

    pub fn create_geo_entity(&mut self, path: &Path, instances: usize, fill: bool, stroke: bool, scale: f32) -> EngineResult<EntityToken> {
        let e = entities::EntityBuilder::new()
            .build_geo(&path, instances, scale, fill, stroke)?;
        self.add_entity(e)
    }

    pub fn add_entity(&mut self, e: GeoEntity) -> EngineResult<EntityToken> {
        self.geo_entities.push(e);
        let index = self.geo_entities.len()-1;
        Ok(EntityToken::new(index, &self.geo_entities[index]))
    }
    
    pub fn init_render(&mut self) {
        info!("Initializing GPU buffers...");
        let device = &self.device;

        let globals_buffer_byte_size = std::mem::size_of::<gpu::Globals>() as u64;
        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Globals ubo"),
            size: globals_buffer_byte_size,
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let mut bind_layouts = vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStage::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(globals_buffer_byte_size),
            },
            count: None,
        }];
        let mut bind_groups = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(globals_buffer.as_entire_buffer_binding()),
        }];
        
        let mut prim_buffers: Vec<wgpu::Buffer> = Vec::new();
        let prim_bind_offset = 1;
        
        for (i, ge) in self.geo_entities.iter().enumerate() {

            let binding = (prim_bind_offset + i) as u32;
            let pbuf_byte_size = (PRIM_BUFFER_LEN * std::mem::size_of::<Primitive>()) as u64;
            
            let pbuf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Prims ubo"),
                size: pbuf_byte_size,
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
                mapped_at_creation: false,
            });

            prim_buffers.push(pbuf);
            // let pbuf2 = prim_buffers.get(i).unwrap();
            


            
            bind_layouts.push(wgpu::BindGroupLayoutEntry{
                binding,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(pbuf_byte_size),
                },
                count: None,
            });


        }

        for (i, pb) in prim_buffers.iter().enumerate() {
            bind_groups.push(wgpu::BindGroupEntry {
                binding: (prim_bind_offset + i) as u32,
                resource: wgpu::BindingResource::Buffer(pb.as_entire_buffer_binding()),
            });
        }

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind group layout"),
            entries: &bind_layouts[..]
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
            label: None,
        });
        
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind group"),
            layout: &bind_group_layout,
            entries: &bind_groups[..],
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

        let render_data = RenderData{
            globals_buffer,
            prim_buffers,
            pipeline_layout,
            bind_group,
            depth_stencil_state,
            sample_count: self.sample_count,
        };

        for geo in self.geo_entities.iter_mut() {
            geo.init_render(device, &render_data);
        }

        for bg in self.bg_entities.iter_mut() {
            bg.init_render(device, &render_data);
        }

        self.render_data = Some(render_data);
    }

    /*
    where
        F: 'static + FnMut(Event<'_, T>, &EventLoopWindowTarget<T>, &mut ControlFlow)
    */

    // pub fn run()
    // fn run_forever(engine: Engine) -> ! {
    //
    // }

    pub fn init_game(&mut self) {

    }

    fn update_tet(&mut self) {
        let current_tet_index = self.scene.curr_tet_index as usize;
        let curr_tet_matrix = &self.scene.curr_tet_matrix;
        let curr_pos = &self.scene.curr_tet_pos;
        for geo in self.geo_entities.iter_mut() {
            for r in 0..4 as usize {
                for c in 0..4 as usize {
                    let mut prim = &mut geo.primitives[PLAYFIELD_SIZE as usize + (r * 4) + c];
                    let solid = curr_tet_matrix.is_solid(c, r);
                    prim.translate = [
                        (curr_pos[0] as f32 + c as f32) * TETRION_SIZE,
                        (curr_pos[1] as f32 + r as f32) * TETRION_SIZE];
                    prim.scale = geo.scale;
                    if solid {
                        prim.color_stroke = [1.0, 1.0, 1.0, 1.0];
                        prim.color = tetrominos::Colors[current_tet_index];
                        prim.width = 0.3;
                        prim.z_index = PLAYFIELD_SIZE as i32;
                    } else {
                        prim.color_stroke = [1.0, 0.0, 1.0, 0.0];
                        prim.color = [1.0, 0.0, 1.0, 0.0];
                    }
                }
            }
        }
    }

    pub fn run(mut self) -> ! {
        let event_loop = self.event_loop.take().unwrap();
        event_loop.run(move |event, _, control_flow| {
            if self.process_input(event, control_flow) {
                return; // Exit loop
            }

            if self.scene.size_changed {
                self.create_scene_textures();
            }

            if let Some(frame) = self.prepare_frame() {
                self.update_state();

                self.render_frame(&frame);

                self.frame_count += 1;
                let now = Instant::now();
                self.anim_secs = (now - self.anim_start).as_secs_f32();
                if now >= self.next_report {
                    // println!("{} FPS", frame_count);
                    self.window.set_title(&*format!("Ruzzle [{:.1}x] {} FPS",
                                                    self.scene.speed,
                                                    self.frame_count));
                    self.frame_count = 0;
                    self.next_report = now + Duration::from_secs(1);
                }
            }
        })
    }

    fn create_scene_textures(&mut self) {
        let mut scene = &mut self.scene;
        let mut swap_chain_desc = &mut self.swap_chain_desc;
        let device = &self.device;
        scene.size_changed = false;
        let physical = scene.window_size;
        swap_chain_desc.width = physical.width;
        swap_chain_desc.height = physical.height;
        self.swap_chain = device.create_swap_chain(&self.surface, &swap_chain_desc);

        warn!("Size changed! New size: {}, {} Updating surface...", physical.width, physical.height);

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth texture"),
            size: wgpu::Extent3d {
                width: swap_chain_desc.width,
                height: swap_chain_desc.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: self.sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        });

        self.depth_texture_view =
            Some(depth_texture.create_view(&wgpu::TextureViewDescriptor::default()));

        self.multisampled_render_target = if self.sample_count > 1 {
            Some(create_multisampled_framebuffer(
                &device,
                &swap_chain_desc,
                self.sample_count,
            ))
        } else {
            None
        };
    }

    fn prepare_frame(&mut self) -> Option<wgpu::SwapChainFrame> {
         match self.swap_chain.get_current_frame() {
            Ok(frame) => Some(frame),
            Err(e) => {
                error!("Swap-chain error: {:?}", e);
                if e == wgpu::SwapChainError::Outdated {
                    self.scene.window_size = self.window.inner_size();
                    self.scene.size_changed = true;
                }
                None
            }
        }
    }

    fn update_state(&mut self) {

        let num_instances = PLAYFIELD_SIZE as usize;
        let fill_prim_id = 0;

        let time_secs = self.anim_secs;

        if !self.scene.curr_tet_active {
            self.scene.curr_tet_index = self.rng.gen_range(1..tetrominos::TL as u8);
            self.scene.curr_tet_pos = [0, 0];
            self.scene.curr_tet_rot = 0;
            self.scene.curr_tet_matrix = tetrominos::ALL[self.scene.curr_tet_index as usize].clone();
            info!("New tetromino: {:?}", tetrominos::NAMES[self.scene.curr_tet_index as usize]);
            self.update_tet();
            self.scene.curr_tet_active = true;
        }

        if self.scene.curr_tet_rot != self.scene.target_tet_rot {
            let curr_tet_shape = &tetrominos::ALL[self.scene.curr_tet_index as usize];
            let new_matrix = curr_tet_shape.rotated(self.scene.target_tet_rot);
            if check_if_free(self.scene.curr_tet_pos, &new_matrix, &self.playfield) {
                self.scene.curr_tet_matrix = new_matrix;
                self.update_tet();
                self.scene.curr_tet_rot = self.scene.target_tet_rot;
            } else {
                // NO! BAD PLAYER
                self.scene.target_tet_rot = self.scene.curr_tet_rot;
            }
        }

        for geo in self.geo_entities.iter_mut() {
            let mut cpu_primitives = &mut geo.primitives;
            for idx in 0..num_instances {
                // let mut cpu_prim = cpu_primitives[(fill_prim_id + idx) as usize];
                // cpu_prim.width = scene.stroke_width;
                let col_offset = ((idx % PLAYFIELD_COLS as usize) as f32 * TETRION_SIZE);
                let row_offset = ((idx / PLAYFIELD_COLS as usize) as f32 * TETRION_SIZE);
                // cpu_primitives[(fill_prim_id + idx) as usize].translate = [
                //     3.0 * ((time_secs * 1.5).sin() * 1.0) + col_offset,
                //     3.0 * ((time_secs * 1.3).sin() * 1.0) + row_offset,
                // ];

                // Stupid "has fill" check
                let w = if cpu_primitives[(fill_prim_id + idx) as usize].color[0..3] == [0.0, 0.0, 0.0] {

                    let wr = (((idx / PLAYFIELD_COLS as usize) as f32
                        - ((time_secs * 0.2).sin().abs() * PLAYFIELD_ROWS as f32)) / PLAYFIELD_ROWS as f32).abs();

                    let wc = (((idx % PLAYFIELD_COLS as usize) as f32
                        - ((time_secs * 0.5).sin().abs() * PLAYFIELD_COLS as f32)) / PLAYFIELD_COLS as f32).abs();

                    //let wr = 1.0;

                    /*
                        ((idx % PLAYFIELD_COLS as usize) as f32 / PLAYFIELD_COLS as f32)
                        + (time_secs * 0.5).sin().abs();
                    */
                    wr * wc
                } else {
                    0.0
                };

                cpu_primitives[idx].color_stroke = [w, w, w, 1.0];// , w, 1.0];


                //cpu_primitives[(stroke_prim_id + idx) as usize].color[0] = (row_offset + (time_secs * 0.3)).sin().abs();
                // cpu_primitives[(stroke_prim_id + idx) as usize].color[1] = (time_secs * 0.01) + (idx as f32).sin().abs();
                //cpu_primitives[(stroke_prim_id + idx) as usize].color[2] = (col_offset + (time_secs * 0.2)).sin().abs();

                if idx == 0{
                 //println!("COLORS: {:.0}%", cpu_primitives[(stroke_prim_id + idx) as usize].color[0] * 100.0);

                }
            }

            let last_down_secs = self.scene.last_down_secs;
            let speed_mod = (2000.0 / self.scene.speed) / 1000.0;

            if (time_secs - last_down_secs) > speed_mod {
                self.scene.last_down_secs = time_secs;
                self.scene.target_piece_y = 1;
            }

            // Update player tile
            if self.scene.target_piece_x != 0 || self.scene.target_piece_y != 0 {

                let mut new_pos = [
                    self.scene.curr_tet_pos[0] + self.scene.target_piece_x as i32,
                    self.scene.curr_tet_pos[1] + self.scene.target_piece_y as i32];

                // if new_pos[0] < 0 || new_pos[0] > (PLAYFIELD_COLS as i8) - 4 {
                //     new_pos[0] = self.scene.curr_tet_pos[0];
                // }
                //
                // if new_pos[1] < 0 || new_pos[1] > (PLAYFIELD_ROWS  as i8) - 4 {
                //     new_pos[1] = self.scene.curr_tet_pos[1];
                // }

                let current_tet_matrix = &self.scene.curr_tet_matrix;

                if check_if_free(new_pos, current_tet_matrix, &self.playfield) {
                    self.scene.curr_tet_pos = new_pos;

                    for r in 0..4 as usize {
                        for c in 0..4 as usize {
                            let mut prim = &mut geo.primitives[PLAYFIELD_SIZE as usize + (r * 4) + c];

                            prim.translate[0] = (new_pos[0] as f32 + c as f32) * TETRION_SIZE;
                            prim.translate[1] = (new_pos[1] as f32 + r as f32) * TETRION_SIZE;
                        }
                    }
                }

                debug!("Player moved {}, {} => {:?}",
                         self.scene.target_piece_x, self.scene.target_piece_y, new_pos);
                self.scene.target_piece_x = 0;
                self.scene.target_piece_y = 0;
            }

        }
    }

    fn render_frame(&mut self, frame: &SwapChainFrame) {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Encoder"),
        });

        let scene = &self.scene;
        let queue = &self.queue;
        let multisampled_render_target = &self.multisampled_render_target;
        let depth_texture_view = &self.depth_texture_view;
        let render_data = self.render_data.as_ref().unwrap();

        queue.write_buffer(
            &render_data.globals_buffer,
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

        for (geo, pbuf) in self.geo_entities.iter()
            .zip(render_data.prim_buffers.iter()) {
            queue.write_buffer(&pbuf, 0, bytemuck::cast_slice(&geo.primitives));
        }

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

        {
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

            for geo in self.geo_entities.iter() {
                let er = geo.renderer.as_ref().unwrap();
                pass.set_pipeline(&er.render_pipeline);
                pass.set_bind_group(0, &render_data.bind_group, &[]);
                pass.set_index_buffer(er.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                pass.set_vertex_buffer(0, er.vertex_buffer.slice(..));

                for range in &geo.ranges {
                //for range in &mut geo.ranges.iter().take(2) {
                    // TODO: what is going on here? where does 154 come from?
                    // let start_instance = if range.start != 0 { 154 } else { 0 };
                    let start_instance = 0;
                    //pass.draw_indexed(range.clone(), 0, 0..(geo.instances as u32)); // geo.instances as u32
                    pass.draw_indexed(range.clone(), 0, start_instance..(start_instance + geo.instances as u32));
                }
            }

            for bg in self.bg_entities.iter() {
                let er = bg.renderer.as_ref().unwrap();
                let instances = 1;
                pass.set_pipeline(&er.render_pipeline);
                pass.set_bind_group(0, &render_data.bind_group, &[]);
                pass.set_index_buffer(er.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                pass.set_vertex_buffer(0, er.vertex_buffer.slice(..));

                for range in &bg.ranges {
                    pass.draw_indexed(range.clone(), 0, 0..instances);
                }
            }
        }


            // if scene.draw_background {
            //     let bg_entity = self.entities.iter().find_map(|e| match e {
            //         Entity::Bg(be) => Some(be),
            //         _ => None,
            //     }).unwrap();
            //
            //     pass.set_pipeline(&bg_entity.render_pipeline);
            //     // pass.set_bind_group(0, &bind_group, &[]);
            //     pass.set_index_buffer(bg_entity.index_buffer.slice(..),
            //                           wgpu::IndexFormat::Uint16);
            //     pass.set_vertex_buffer(0, bg_entity.vertex_buffer.slice(..));
            //
            //
            //     pass.draw_indexed(0..6, 0, 0..1);
            // }


        queue.submit(Some(encoder.finish()));
    }

    pub fn process_input(&mut self, event: winit::event::Event<()>, control_flow: &mut ELoop::ControlFlow) -> bool {
        let mut scene = &mut self.scene;
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
                *control_flow = ELoop::ControlFlow::Exit;
                return false;
            }
            Event::WindowEvent {
                event: WindowEvent::MouseWheel { delta, .. },
                ..
            } => {
                let ydelta = match delta {
                    winit::event::MouseScrollDelta::LineDelta( _, y ) => y as f32,
                    winit::event::MouseScrollDelta::PixelDelta( pp) => pp.y as f32
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
                    winit::event::KeyboardInput {
                        state: winit::event::ElementState::Pressed,
                        virtual_keycode: Some(key),
                        ..
                    },
                    ..
                },
                ..
            } => match key {
                VirtualKeyCode::Escape => {
                    *control_flow = ELoop::ControlFlow::Exit;
                    return false;
                }
                VirtualKeyCode::PageDown => {
                    scene.target_zoom *= 0.8;
                }
                VirtualKeyCode::PageUp => {
                    scene.target_zoom *= 1.25;
                }
                VirtualKeyCode::Minus| VirtualKeyCode::O => {
                    scene.speed *= 0.8;
                }
                VirtualKeyCode::Plus | VirtualKeyCode::P => {
                    scene.speed *= 1.25;
                }
                VirtualKeyCode::Left => {
                    scene.target_piece_x = -1;
                    //scene.target_scroll.x -= 50.0 / scene.target_zoom;
                }
                VirtualKeyCode::Right => {
                    scene.target_piece_x = 1;

//                    scene.target_scroll.x += 50.0 / scene.target_zoom;
                }
                VirtualKeyCode::Up => {
                    scene.target_piece_y = -1;
                    //scene.target_scroll.y -= 50.0 / scene.target_zoom;
                }
                VirtualKeyCode::Down => {
                    scene.target_piece_y = 1;
                    //scene.target_scroll.y += 50.0 / scene.target_zoom;
                }
                VirtualKeyCode::Return => {
                    scene.curr_tet_active = false;
                }
                VirtualKeyCode::Space | VirtualKeyCode::X => {
                    scene.target_tet_rot = (scene.curr_tet_rot + 1) % 4;
                }
                VirtualKeyCode::Back | VirtualKeyCode::Z => {
                    scene.target_tet_rot = ((4 + scene.curr_tet_rot) - 1) % 4;
                }
                // VirtualKeyCode::P => {
                //     scene.show_points = !scene.show_points;
                // }
                VirtualKeyCode::B => {
                    scene.draw_background = !scene.draw_background;
                }
                VirtualKeyCode::A => {
                    scene.target_stroke_width /= 0.8;
                }
                // VirtualKeyCode::Z => {
                //     scene.target_stroke_width *= 0.8;
                // }
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

        *control_flow = ELoop::ControlFlow::Poll;

        true // Keep looping
    }
}

fn check_if_free(pos: [i32 ; 2], tetro_shape: &TetroShape, blocks: &[usize ; PLAYFIELD_SIZE as usize]) -> bool {
    for r in 0..4 as usize {
        let offset_row = r as i32 + pos[1];
        let block_row = offset_row * PLAYFIELD_COLS as i32;
        for c in 0..4 as usize {
            let offset_col = c as i32 + pos[0];
            if match tetro_shape {
                TetroShape::Odd(t) if c < 3 && r < 3 => t[r][c],
                TetroShape::Even(t) => t[r][c],
                _ => false
            } && (
                // Check if outside play field
                offset_row < 0 || offset_row >= PLAYFIELD_ROWS as i32 ||
                offset_col < 0 || offset_col >= PLAYFIELD_COLS as i32 ||
                // Check if target block is occupied
                blocks[block_row as usize + (offset_col as usize)] != 0) { return false; }
        }
    }
    return true;
}


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
    target_piece_x: i8,
    target_piece_y: i8,
    target_tet_rot: u8,
    curr_tet_pos: [i32 ; 2],
    curr_tet_index: u8,
    curr_tet_active: bool,
    curr_tet_rot: u8,
    curr_tet_matrix: tetrominos::TetroShape,
    speed: f32,
    last_down_secs: f32,
}