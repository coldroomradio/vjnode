use std::sync::Arc;
use winit::{
    event::*,
    event_loop::EventLoop,
    window::WindowBuilder,
};

mod audio;
mod graph;
mod nodes;
mod state;
mod ui;
use audio::AudioAnalyzer;
use state::State;

fn main() {
    env_logger::init();

    // Starts capturing from the default input device immediately and
    // keeps analyzing in the background for as long as `audio` is alive.
    let audio = AudioAnalyzer::start();

    let event_loop = EventLoop::new().unwrap();
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("vjnode")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
            .build(&event_loop)
            .unwrap(),
    );

    let mut state = pollster::block_on(State::new(window.clone(), audio.bands.clone()));

    event_loop
        .run(move |event, elwt| match event {
            Event::WindowEvent { window_id, event } if window_id == state.window().id() => {
                state.handle_window_event(&event);
                match event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::Resized(size) => state.resize(size),
                    WindowEvent::RedrawRequested => {
                        state.update();
                        match state.render() {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => state.resize(state.size()),
                            Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                            Err(e) => eprintln!("render error: {:?}", e),
                        }
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                // Continuous redraw loop — this is what gives you real-time
                // rendering rather than redraw-on-demand.
                state.window().request_redraw();
            }
            _ => {}
        })
        .unwrap();
}
