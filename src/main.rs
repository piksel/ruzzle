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
use ruzzle::tetrominos::{Tetromino, TetroShape};
use rand::Rng;
use ruzzle::{tetrominos};
use ruzzle::engine::*;
use log::{Log, info};

#[macro_use]
extern crate lazy_static;

//use log;



const DEFAULT_WINDOW_WIDTH: f32 = 1024.0;
const DEFAULT_WINDOW_HEIGHT: f32 = 768.0;

const PLAYFIELD_COLS: u32 = 10;
const PLAYFIELD_ROWS: u32 = 16; // 16
const PLAYFIELD_SIZE: u32 = PLAYFIELD_COLS * PLAYFIELD_ROWS;
const TETRION_SIZE: f32 = 8.0;
const MAX_CURR_SIZE:usize = 16;

// Number of samples for anti-aliasing
// Set to 1 to disable
const SAMPLE_COUNT: u32 = 4;
const TOLERANCE: f32 = 0.02;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        //.filter_module("ruzzle", log::LevelFilter::Debug)
        .filter_module("ruzzle", log::LevelFilter::Info)
        .init();

    let config = ruzzle::config::load_config();

    println!();
    println!(" RUZZLE alpha");
    println!(" https://github.com/piksel/ruzzle");
    println!();
    println!(" Controls:");
    println!("   Arrow keys  : move current tetromino");
    println!("   X/Space     : rotate current tetromino clockwise");
    println!("   Z/Backspace : rotate current tetromino counter clockwise");
    println!("   PgUp/PgDown : zoom in/out (or mouse wheel)");
    println!("           +/- : increase/decrease level");
    println!();

    let mut engine = Engine::new(
        config.graphics.sample_count,
        config.graphics.tolerance,
        config.graphics.use_low_power_gpu);

    let num_instances: usize = PLAYFIELD_SIZE as usize + MAX_CURR_SIZE;

    let tetrion_path_scale = 0.8;
    //build_tetrion_path(&mut builder);

    let mut builder = Path::builder().with_svg();
    build_tetrion_path(&mut builder);
    let path = builder.build();

    engine.create_geo_entity(
        &path, num_instances, true, true, tetrion_path_scale
    );

    engine.create_bg_entity(
        2.0,
    );

    // /!\ No more entities can be added after this step
    engine.init_render();


    let mut index = 0;

    // Set up the initial play field
    for r in 0..PLAYFIELD_ROWS as usize {
        let rb = r / 4;
        let ri = r % 4;
        for c in 0..PLAYFIELD_COLS as usize {
            // Hacky way to show all tetrominos; flip if past middle to add "padding"
            let tetind = if c < 5 { rb } else { 4 + rb };
            let ci = if c < 5 { c } else { (PLAYFIELD_COLS as usize - c)-1 };
            let tetromino = &tetrominos::ALL[tetind];
            engine.playfield[index] = if tetromino.is_solid(ci, ri) { tetind } else { 0 };
            index += 1;
        }
    }

    engine.init_game();


    // Update entities from play field
    for geo in engine.geo_entities.iter_mut() {
        for (idx, (prim, tet)) in geo.primitives
            .iter_mut()
            .zip(engine.playfield.iter())
            .enumerate()
            .take(PLAYFIELD_SIZE as usize)
        {
            let color = tetrominos::Colors[*tet];
            let idf = idx as f32;
            let col = idf % (PLAYFIELD_COLS as f32);
            let row = (idf / (PLAYFIELD_COLS as f32)).floor();
            prim.translate = [
                col * TETRION_SIZE,
                row * TETRION_SIZE,
            ];
            prim.z_index = (idx as u32 + 1) as i32;

            if *tet == 0 {
                prim.color = [0.0, 0.0, 0.0, 1.0];
            } else {
                prim.color = color;
            }

            prim.scale = tetrion_path_scale;
            prim.width = 0.3;
        }


    }


    info!("Starting main loop!");
    engine.run();
}

