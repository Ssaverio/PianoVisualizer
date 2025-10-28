mod config;
mod note;
mod state;

use pollster::block_on;
use state::State;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Piano Visualizer")
        .with_inner_size(winit::dpi::PhysicalSize::new(800, 600))
        .build(&event_loop)
        .unwrap();

    let mut state = block_on(State::new(&window));

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(size) => state.resize(*size),
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    state.resize(**new_inner_size)
                }
                _ => {}
            },
            Event::RedrawRequested(_) => {
                state.update();
                state.render();
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => {}
        }
    });
}
