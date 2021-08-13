#![warn(clippy::all, rust_2018_idioms)]

mod camera;
mod face;
mod import;
mod interface;
mod state;

use camera::{Camera, CameraController};
use chrono::Timelike;
use egui_winit_platform::Platform;
use eyre::Result;
use futures_lite::future::block_on;
use interface::Interface;
use std::path::PathBuf;
use structopt::StructOpt;
use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

const INITIAL_WIDTH: u32 = 1920;
const INITIAL_HEIGHT: u32 = 1080;

#[derive(Debug, StructOpt)]
#[structopt(name = "ennona", about = "Point cloud viewer for rust-cv")]
struct Opt {
    /// Activate debug mode
    #[structopt(short, long)]
    debug: bool,

    /// Input file (ply)
    #[structopt(parse(from_os_str))]
    input_file: Option<PathBuf>,
}

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<()> {
    use std::time::Instant;

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
        .with_title("⛅ Ennona")
        .with_inner_size(winit::dpi::PhysicalSize {
            width: INITIAL_WIDTH,
            height: INITIAL_HEIGHT,
        })
        .build(&event_loop)?;

    window.set_window_icon(None);

    let mut state = block_on(state::State::new(&window));
    let mut camera = state.create_initial_camera();
    let mut app = Interface::new(
        "".into(),
        window.inner_size().height,
        window.inner_size().width,
    );

    if let Some(f) = opt.input_file {
        let import = import::import(&f)?;
        if let import::Import::Ply(gpu_data) = import {
            let avg_pos = import::avg_vertex_position(&gpu_data.point_vertices);
            let avg_dist = import::avg_vertex_distance(avg_pos, &gpu_data.point_vertices);

            camera.set_camera_facing(avg_pos, avg_dist * 5.0);
            app.set_camera_scale(avg_dist);

            state.import_ply_data(&gpu_data);
        } else {
            log::warn!("Ignoring `input_file` option. Can't parse as PLY.");
        }
    }

    let mut last_render_time = Instant::now();

    event_loop.run(move |event: Event<'_, ()>, _, control_flow| {
        state.platform.handle_event(&event);

        match event {
            Event::RedrawRequested(_) => {
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;
                app.update_camera(&mut camera, dt);
                state.update(&camera);

                match state.render(&mut app, window.scale_factor()) {
                    Ok(_) => {}
                    // Recreate the swap_chain if lost
                    Err(wgpu::SwapChainError::Lost) => {
                        app.resize(window.inner_size());
                        camera.resize(window.inner_size());
                        state.rebuild_swapchain(window.inner_size());
                    }
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SwapChainError::OutOfMemory) => *control_flow = ControlFlow::Exit,

                    Err(wgpu::SwapChainError::Outdated) => {
                        app.resize(window.inner_size());
                        camera.resize(window.inner_size());
                        state.rebuild_swapchain(window.inner_size());
                    }
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("107 {:?}", e),
                }
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::DeviceEvent { .. } => {
                app.input(&event, &window);
            }
            Event::WindowEvent {
                event: ref window_event,
                window_id,
                ..
            } if window_id == window.id() => {
                match window_event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::Resized(_) => {
                        app.resize(window.inner_size());
                        camera.resize(window.inner_size());
                        state.rebuild_swapchain(window.inner_size());
                    }
                    WindowEvent::ScaleFactorChanged { .. } => {
                        app.resize(window.inner_size());
                        camera.resize(window.inner_size());
                        state.rebuild_swapchain(window.inner_size());
                    }
                    WindowEvent::DroppedFile(path) => {
                        match import::import(path) {
                            Ok(imported) => match imported {
                                import::Import::Ply(ply_data) => {
                                    let avg_pos =
                                        import::avg_vertex_position(&ply_data.point_vertices);
                                    let avg_dist = import::avg_vertex_distance(
                                        avg_pos,
                                        &ply_data.point_vertices,
                                    );

                                    camera.set_camera_facing(avg_pos, avg_dist * 5.0);
                                    app.set_camera_scale(avg_dist);

                                    state.import_ply_data(&ply_data);
                                }
                                import::Import::Image(img) => {
                                    state.import_image(img, &mut app);
                                }
                            },
                            Err(e) => eprintln!("145 {:?}", e),
                        };
                    }
                    _ => {}
                }

                if !state.platform.captures_event(&event) {
                    app.input(&event, &window);
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
