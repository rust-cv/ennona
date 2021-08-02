#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod camera;
mod import;
mod state;

use app::Application;
use camera::{Camera, CameraController};
use chrono::Timelike;
use egui_winit_platform::Platform;
use eyre::Result;
use futures_lite::future::block_on;
use std::path::PathBuf;
use structopt::StructOpt;
use winit::event::Event;
use winit::event::WindowEvent;
use winit::event_loop::ControlFlow;

const INITIAL_WIDTH: u32 = 1920;
const INITIAL_HEIGHT: u32 = 1080;

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
    use std::time::Instant;

    use log::info;

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

    let mut last_render_time = Instant::now();

    event_loop.run(move |event: Event<'_, ()>, _, control_flow| {
        state.platform.handle_event(&event);
        if captures_event(&state.platform, &event) {
            info!("captured");
        }
        match event {
            Event::RedrawRequested(_) => {
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;
                state.update(dt);

                match state.render(window.scale_factor()) {
                    Ok(_) => {}
                    // Recreate the swap_chain if lost
                    Err(wgpu::SwapChainError::Lost) => state.resize(state.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SwapChainError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::DeviceEvent {
                ref event,
                .. // We're not using device_id currently
            } => {
                state.input(event);
            }
            Event::WindowEvent {
                ref event,
                window_id,
                ..
            } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::Resized(size) => {
                        state.resize(*size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }

                    _ => {}
                }
            }
            _ => (),
        }
    });
}

/// Time of day as seconds since midnight.
pub fn seconds_since_midnight() -> f64 {
    let time = chrono::Local::now().time();
    time.num_seconds_from_midnight() as f64 + 1e-9 * (time.nanosecond() as f64)
}

pub fn captures_event<T>(platform: &Platform, winit_event: &Event<'_, T>) -> bool {
    match winit_event {
        Event::WindowEvent {
            window_id: _window_id,
            event,
        } => match event {
            WindowEvent::ReceivedCharacter(_)
            | WindowEvent::KeyboardInput { .. }
            | WindowEvent::ModifiersChanged(_) => platform.context().wants_keyboard_input(),

            WindowEvent::MouseWheel { .. } | WindowEvent::MouseInput { .. } => {
                platform.context().wants_pointer_input()
            }

            WindowEvent::CursorMoved { .. } => platform.context().is_using_pointer(),

            _ => false,
        },

        _ => false,
    }
}
