#![feature(drain_filter)]

mod gpu;
mod sim;

use futures::executor::block_on;
use gpu::Vertex;
use gridsim::SquareGrid;
use itertools::Itertools;
use ndarray::{Array2, Zip};
use rand::Rng;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use sim::Abiogenesis;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let mut state = block_on(gpu::State::new(&window));

    let mut grid = SquareGrid::new(Abiogenesis, Array2::default((20, 32)));

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => match event {
                // Close
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                // Keyboard input
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => *control_flow = ControlFlow::Exit,
                // Window resize
                WindowEvent::Resized(physical_size) => {
                    state.resize(*physical_size);
                }
                // Window scale change
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    state.resize(**new_inner_size);
                }
                // Any other event
                _ => {}
            },
            Event::RedrawRequested(_) => {
                grid.step_parallel();
                state.update(&populate_all_particles(&grid));
                match state.render() {
                    Ok(_) => {}
                    // Recreate the swap_chain if lost
                    Err(wgpu::SwapChainError::Lost) => state.resize(state.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SwapChainError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(wgpu::SwapChainError::Outdated) | Err(wgpu::SwapChainError::Timeout) => {}
                }
                let mut thread_lengths = grid
                    .cells()
                    .iter()
                    .flat_map(|c| c.iter().map(|p| p.thread.len()))
                    .counts()
                    .into_iter()
                    .collect::<Vec<_>>();
                thread_lengths.sort_unstable_by_key(|&(len, _)| len);
                println!("{:?}", thread_lengths);
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually request it.
                window.request_redraw();
            }
            _ => {}
        }
    });
}

fn populate_all_particles(grid: &SquareGrid<Abiogenesis>) -> Vec<Vertex> {
    let cells = grid.cells();
    let (height, width) = cells.dim();

    let total_cells: usize = cells.into_par_iter().map(|cell| cell.len()).sum();

    println!("cells: {}", total_cells);

    Zip::indexed(cells)
        .into_par_iter()
        .flat_map(|((y, x), cell)| {
            cell.par_iter().map(move |particle| {
                let mut rng = particle.thread.len_rng();

                let color = palette::Hsv::new(
                    palette::RgbHue::from_degrees(rng.gen_range(0.0..360.0)),
                    1.0,
                    1.0,
                );
                let color: palette::LinSrgb = color.into();
                let (red, green, blue) = color.into();
                let v = particle.motion.position;
                Vertex {
                    position: [
                        (v.x as f32 + x as f32) / width as f32 * 2.0 - 1.0,
                        (v.y as f32 + y as f32) / height as f32 * 2.0 - 1.0,
                    ],
                    color: [red, green, blue],
                }
            })
        })
        .collect()
}
