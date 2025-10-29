mod config;
mod midi_loader;
mod state;
mod vertex;

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
            } if window_id == window.id() => {
                let egui_consumed_event =
                    state.egui_state.on_event(&state.egui_ctx, event).consumed;

                if !egui_consumed_event {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(size) => state.resize(*size),
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            state.resize(**new_inner_size)
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(_) => {
                state.update();

                // --- COSTRUZIONE UI EGUI (MODIFICATA) ---
                let mut raw_input = state.egui_state.take_egui_input(&window);
                
                // Il fix per il DPI
                raw_input.pixels_per_point = Some(window.scale_factor() as f32);
                
                let full_output = state.egui_ctx.run(raw_input, |ctx| {
                    // Qui costruiamo la nostra UI
                    egui::Window::new("Impostazioni").show(ctx, |ui| {
                        ui.label("Velocità Animazione");
                        ui.add(
                            egui::Slider::new(&mut state.fall_duration_secs, 0.5..=10.0)
                                .text("Durata Caduta (sec)"),
                        );
                        ui.label("(Valori più bassi = più veloce)");
                        
                        ui.separator(); // Un separatore visivo

                        ui.label("Colori Note");
                        ui.horizontal(|ui| {
                            ui.label("Mano Sinistra:");
                            // --- MODIFICA QUI ---
                            // Usiamo 'srgba' che accetta &mut Color32
                            egui::color_picker::color_edit_button_srgba(ui, &mut state.color_left_hand, egui::color_picker::Alpha::Opaque);
                            // --- FINE MODIFICA ---
                        });
                        ui.horizontal(|ui| {
                            ui.label("Mano Destra:");
                            // --- MODIFICA QUI ---
                            // Usiamo 'srgba' che accetta &mut Color32
                            egui::color_picker::color_edit_button_srgba(ui, &mut state.color_right_hand, egui::color_picker::Alpha::Opaque);
                            // --- FINE MODIFICA ---
                        });
                        ui.label("(Split su Do Centrale - Tasto 60)");
                    });
                });
                
                state
                    .egui_state
                    .handle_platform_output(&window, &state.egui_ctx, full_output.platform_output);

                let egui_primitives = state.egui_ctx.tessellate(full_output.shapes);
                let egui_textures_delta = full_output.textures_delta;
                // --- FINE COSTRUZIONE EGUI ---

                state.render(&window, &egui_primitives, &egui_textures_delta);
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => {}
        }
    });
}