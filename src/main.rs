mod camera;
mod import;
mod interface;
mod points;
mod state;

use camera::{Camera, CameraController};
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
    use std::time::{Duration, Instant};

    use image::GenericImageView;
    use wgpu::SurfaceError;
    use winit::dpi::PhysicalPosition;

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
    let mut camera = Camera::new(window.inner_size());
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

            state.import_ply(&gpu_data);
        } else {
            log::warn!("Ignoring `input_file` option. Can't parse as PLY.");
        }
    }

    let mut last_update_time = Instant::now();
    let mut last_render_time = last_update_time;
    let mut mouse_position: Option<PhysicalPosition<f64>> = None;

    event_loop.run(move |event: Event<'_, ()>, _, control_flow| {
        // Handle GUI events and if the GUI captures the event, we do not want to handle it ourselves,
        // so return immediately in that case.
        if state.handle_event(&event) {
            return;
        }

        // Perform our own handling of events.
        match event {
            Event::RedrawRequested(_) => {
                match state.render(&mut app, &camera, window.scale_factor()) {
                    Ok(_) => {
                        last_render_time = last_update_time;
                    }
                    // Resize surface if lost (not sure how) or outdated (probably synchronization error)
                    Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                        app.resize(window.inner_size());
                        camera.resize(window.inner_size());
                        state.resize(window.inner_size());
                        window.request_redraw();
                    }
                    // The system is out of memory, we should probably quit
                    Err(SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // If there is a timeout, we should just request another redraw
                    // and hopefully it will be able to draw next frame.
                    Err(SurfaceError::Timeout) => {
                        eprintln!("warning: there was a timeout");
                        window.request_redraw();
                    }
                }
            }
            Event::MainEventsCleared => {
                let now = Instant::now();
                let dt = now - last_update_time;
                last_update_time = now;
                app.update_camera(&mut camera, dt);
                if last_render_time.elapsed() >= Duration::from_millis(15) {
                    window.request_redraw();
                }
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
                        state.resize(window.inner_size());
                    }
                    WindowEvent::ScaleFactorChanged { .. } => {
                        app.resize(window.inner_size());
                        camera.resize(window.inner_size());
                        state.resize(window.inner_size());
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

                                    state.import_ply(&ply_data);
                                }
                                import::Import::Image(image) => {
                                    let size = (image.width() as f32, image.height() as f32);
                                    let texture = state.make_egui_texture(image);
                                    app.add_image(texture, size);
                                }
                            },
                            Err(e) => eprintln!("145 {:?}", e),
                        };
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        if let Some(old_pos) = mouse_position.replace(*position) {
                            let delta_x = position.x - old_pos.x;
                            let delta_y = position.y - old_pos.y;
                            if app.camera_controller.mouse_captured {
                                let size = window.inner_size();
                                let center = PhysicalPosition {
                                    x: size.width / 2,
                                    y: size.height / 2,
                                };

                                if window.set_cursor_position(center).is_ok() {
                                    mouse_position.replace(PhysicalPosition {
                                        x: center.x as f64,
                                        y: center.y as f64,
                                    });
                                }
                                app.camera_controller
                                    .process_mouse(&mut camera, delta_x, delta_y);
                            }
                        }
                    }
                    _ => {}
                }

                app.input(window_event, &window);
            }
            _ => (),
        }
    });
}
