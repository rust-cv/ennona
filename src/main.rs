#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod camera;
mod import;
mod state;

use app::Application;
use camera::Camera;
use chrono::Timelike;
use eyre::Result;
use futures_lite::future::block_on;
use std::path::PathBuf;
use structopt::StructOpt;
use winit::event::WindowEvent;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode};
use winit::event_loop::ControlFlow;

const INITIAL_WIDTH: u32 = 800;
const INITIAL_HEIGHT: u32 = 600;

#[derive(Debug, StructOpt)]
#[structopt(name = "ennona", about = "Point cloud viewer for rust-cv")]
struct Opt {
    /// Input file (ply)
    #[structopt(parse(from_os_str))]
    input_file: PathBuf,

    /// Activate debug mode
    #[structopt(short, long)]
    debug: bool,
}

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<()> {
    let opt = Opt::from_args();

    pretty_env_logger::formatted_builder()
        .filter_level(if opt.debug {
            log::LevelFilter::Info
        } else {
            log::LevelFilter::Warn
        })
        .init();

    let event_loop = winit::event_loop::EventLoop::with_user_event();
    let window = winit::window::WindowBuilder::new()
        .with_decorations(true)
        .with_resizable(true)
        .with_transparent(false)
        .with_title("⛅ Cloud")
        .with_inner_size(winit::dpi::PhysicalSize {
            width: INITIAL_WIDTH,
            height: INITIAL_HEIGHT,
        })
        .build(&event_loop)?;

    window.set_window_icon(None);

    let mut state = block_on(state::State::new(&window));

    let points = import::import(opt.input_file)?;
    let avg_pos = import::avg_vertex_position(&points);
    let avg_dist = import::avg_vertex_distance(avg_pos, &points);

    state.set_start_position(avg_pos, avg_dist);

    state.import_vertices(&points);

    event_loop.run(move |event: Event<'_, ()>, _, control_flow| {
        state.platform.handle_event(&event);
        match event {
            Event::RedrawRequested(_) => {
                state.update();

                state.render(window.scale_factor()).unwrap();

                *control_flow = ControlFlow::Poll;
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::WindowEvent {
                ref event,
                window_id,
                ..
            } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::Resized(size) => {
                    state.resize(*size);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    state.resize(**new_inner_size);
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => *control_flow = ControlFlow::Exit,

                _ => {}
            },
            _ => (),
        }
    });
}

/// Time of day as seconds since midnight.
pub fn seconds_since_midnight() -> f64 {
    let time = chrono::Local::now().time();
    time.num_seconds_from_midnight() as f64 + 1e-9 * (time.nanosecond() as f64)
}
