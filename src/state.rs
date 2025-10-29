// state.rs
use crate::config::*;
use crate::midi_loader::{self, MidiNote};
use crate::vertex::Vertex;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use bytemuck::{Pod, Zeroable};
use egui::Color32; // <--- AGGIUNTO
use std::time::Instant;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct StateUniforms {
    screen_size: [f32; 2],
}

pub struct State {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: PhysicalSize<u32>,

    pub midi_notes: Vec<MidiNote>,
    pub start_time: Instant,

    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub num_vertices: u32,

    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub uniform_bind_group_layout: wgpu::BindGroupLayout,

    pub egui_ctx: egui::Context,
    pub egui_state: egui_winit::State,
    pub egui_renderer: egui_wgpu::Renderer,
    pub fall_duration_secs: f32,

    // --- CAMPI AGGIUNTI PER I COLORI ---
    pub color_left_hand: Color32,
    pub color_right_hand: Color32,
}

impl State {
    pub async fn new(window: &winit::window::Window) -> Self {
        let size = window.inner_size();

        // --- WGPU inizializzazione ---
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });
        let surface = unsafe { instance.create_surface(window) }.unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // --- Caricamento Dati ---
        let midi_path = std::path::Path::new("test.mid");
        let midi_notes = if midi_path.exists() {
            midi_loader::load_midi_file(midi_path)
        } else {
            println!(
                "[ATTENZIONE] File MIDI di test '{}' non trovato, uso dati di fallback.",
                midi_path.display()
            );
            vec![
                MidiNote {
                    pitch: 60,
                    velocity: 100,
                    start_time_secs: 2.0,
                    duration_secs: 1.0,
                },
                MidiNote {
                    pitch: 62,
                    velocity: 100,
                    start_time_secs: 3.0,
                    duration_secs: 0.5,
                },
                MidiNote {
                    pitch: 64,
                    velocity: 100,
                    start_time_secs: 4.0,
                    duration_secs: 1.5,
                },
                // Aggiungiamo una nota per la mano sinistra per test
                MidiNote {
                    pitch: 48, // Sotto il Do centrale
                    velocity: 100,
                    start_time_secs: 2.5,
                    duration_secs: 1.0,
                },
            ]
        };
        println!("Caricate {} note.", midi_notes.len());
        let start_time = Instant::now();

        // --- Creazione Uniforms ---
        let uniforms = StateUniforms {
            screen_size: [size.width as f32, size.height as f32],
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Uniform Bind Group Layout"),
            });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("Uniform Bind Group"),
        });

        // --- Shader ---
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // --- Pipeline ---
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout],
                push_constant_ranges: &[],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // --- Vertex Buffer ---
        let initial_size = 6 * 10 * std::mem::size_of::<Vertex>() as u64;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: initial_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // --- INIZIALIZZAZIONE EGUI ---
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(window);
        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            config.format,
            None,
            1,
        );

        // 7. Aggiungiamo i nuovi campi a Self
        Self {
            surface,
            device,
            queue,
            config,
            size,
            midi_notes,
            start_time,
            render_pipeline,
            vertex_buffer,
            num_vertices: 0,
            uniform_buffer,
            uniform_bind_group,
            uniform_bind_group_layout,
            egui_ctx,
            egui_state,
            egui_renderer,
            fall_duration_secs: FALL_DURATION_SECS,

            // --- INIZIALIZZAZIONE COLORI ---
            color_left_hand: Color32::from_rgb(0, 100, 255), // Un bel blu
            color_right_hand: Color32::from_rgb(0, 255, 100), // Un bel verde
        }
    }

    // --- FUNZIONE RESIZE (invariata) ---
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            let uniforms = StateUniforms {
                screen_size: [new_size.width as f32, new_size.height as f32],
            };
            self.queue.write_buffer(
                &self.uniform_buffer,
                0,
                bytemuck::bytes_of(&uniforms),
            );
        }
    }

    // --- FUNZIONE UPDATE (MODIFICATA) ---
    pub fn update(&mut self) {
        let current_time_secs = self.start_time.elapsed().as_secs_f32();
        let screen_height = self.size.height as f32;
        let screen_width = self.size.width as f32;
        
        let pixels_per_second = screen_height / self.fall_duration_secs;

        // --- PRENDIAMO I COLORI DALLO STATO ---
        // Convertiamo da Color32 (0-255) a [f32; 3] (0.0-1.0) per lo shader
let color_lh_f32: [f32; 3] = [
        self.color_left_hand.r() as f32 / 255.0,
        self.color_left_hand.g() as f32 / 255.0,
        self.color_left_hand.b() as f32 / 255.0,
    ];
    let color_rh_f32: [f32; 3] = [
        self.color_right_hand.r() as f32 / 255.0,
        self.color_right_hand.g() as f32 / 255.0,
        self.color_right_hand.b() as f32 / 255.0,
    ];
        const MIDDLE_C_PITCH: u8 = 60; // Do centrale

        let mut vertices = Vec::new();

        for note in &self.midi_notes {
            let present_line_y = 0.0;
            let y_hit_position =
                present_line_y + (note.start_time_secs - current_time_secs) * pixels_per_second;
            let note_height_pixels = note.duration_secs * pixels_per_second;
            let y_top_position = y_hit_position + note_height_pixels;

            if y_top_position < 0.0 || y_hit_position > screen_height {
                continue;
            }

            let x_pos = (note.pitch as f32 - 48.0) * NOTE_WIDTH + (screen_width / 4.0);
            let w = NOTE_WIDTH;
            
            // --- LOGICA COLORE MODIFICATA ---
            // Rimuoviamo la costante
            // let c = NOTE_COLOR; // <-- RIMOSSO
            
            // Scegliamo il colore in base all'altezza (pitch) della nota
            let c = if note.pitch < MIDDLE_C_PITCH {
                color_lh_f32
            } else {
                color_rh_f32
            };
            // --- FINE MODIFICA ---

            let y_top = screen_height - y_top_position;
            let y_hit = screen_height - y_hit_position;

            vertices.extend_from_slice(&[
                Vertex {
                    position: [x_pos, y_hit],
                    color: c,
                },
                Vertex {
                    position: [x_pos + w, y_hit],
                    color: c,
                },
                Vertex {
                    position: [x_pos, y_top],
                    color: c,
                },
                Vertex {
                    position: [x_pos + w, y_hit],
                    color: c,
                },
                Vertex {
                    position: [x_pos + w, y_top],
                    color: c,
                },
                Vertex {
                    position: [x_pos, y_top],
                    color: c,
                },
            ]);
        }

        // ... (Gestione buffer invariata) ...
        if !vertices.is_empty() {
            let required_size = (vertices.len() * std::mem::size_of::<Vertex>()) as u64;
            let current_size = self.vertex_buffer.size();

            if required_size > current_size {
                let new_size = required_size * 2;
                println!(
                    "[INFO] Riallocazione buffer GPU: {} -> {} bytes",
                    current_size, new_size
                );

                self.vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Vertex Buffer (Dynamic)"),
                    size: new_size,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }

            self.queue
                .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
            self.num_vertices = vertices.len() as u32;
        } else {
            self.num_vertices = 0;
        }
    }

    // --- FUNZIONE RENDER (invariata) ---
    pub fn render(
        &mut self,
        window: &Window,
        egui_primitives: &[egui::ClippedPrimitive],
        egui_textures_delta: &egui::TexturesDelta,
    ) {
        let output = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                eprintln!("Errore get_current_texture: {:?}", e);
                if e == wgpu::SurfaceError::Lost {
                    self.resize(self.size);
                }
                return;
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let screen_descriptor = egui_wgpu::renderer::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: window.scale_factor() as f32,
        };

        for (id, image_delta) in &egui_textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }
        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            egui_primitives,
            &screen_descriptor,
        );
        
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            // Disegna la tua visualizzazione
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..self.num_vertices, 0..1);

            // Disegna egui
            self.egui_renderer
                .render(&mut render_pass, egui_primitives, &screen_descriptor);
        }

        for id in &egui_textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}