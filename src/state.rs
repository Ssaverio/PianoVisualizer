// state.rs
use crate::config::*;
use crate::midi_loader::{self, MidiNote};
use crate::vertex::Vertex;
use wgpu::util::DeviceExt; // Importa 'DeviceExt' per 'create_buffer_init'
use winit::dpi::PhysicalSize;

use bytemuck::{Pod, Zeroable};
use std::time::Instant; // Aggiunto: Ne abbiamo bisogno per la struct Uniform

// 1. Definiamo la struct Rust che *corrisponde* a quella WGSL
// #[repr(C)] assicura che Rust disponga i dati in memoria
// in modo compatibile con C (e quindi con lo shader).
// Pod e Zeroable servono a bytemuck per convertire
// questa struct in un array di byte (`&[u8]`).
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

    // 2. Aggiungiamo i campi per gestire i nostri Uniforms
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    // Dobbiamo salvare anche il 'layout' per creare la pipeline
    pub uniform_bind_group_layout: wgpu::BindGroupLayout,
}

impl State {
    pub async fn new(window: &winit::window::Window) -> Self {
        let size = window.inner_size();

        // --- WGPU inizializzazione ---
        // ... (Questa parte è invariata) ...
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
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_capabilities(&adapter).formats[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // --- Caricamento Dati ---
        // ... (Questa parte è invariata) ...
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
            ]
        };
        println!("Caricate {} note.", midi_notes.len());
        let start_time = Instant::now();

        // 3. --- Creazione Uniforms ---

        // Crea l'istanza iniziale dei dati
        let uniforms = StateUniforms {
            screen_size: [size.width as f32, size.height as f32],
        };

        // Crea il buffer sulla GPU e copia subito i nostri dati
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            // Converti la struct in byte
            contents: bytemuck::bytes_of(&uniforms),
            // UNIFORM: Dati per lo shader
            // COPY_DST: Possiamo aggiornarlo in `resize()`
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 4. Definiamo il "layout" del Bind Group.
        // Questo descrive *cosa* stiamo collegando.
        // Deve corrispondere a `@group(0) @binding(0)` nello shader.
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,                             // Corrisponde a @binding(0)
                    visibility: wgpu::ShaderStages::VERTEX, // Visibile solo nel vertex shader
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Uniform Bind Group Layout"),
            });

        // 5. Creiamo il "Bind Group"
        // Questo è l'oggetto che *collega* il layout (lo "schema")
        // al buffer (i "dati").
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("Uniform Bind Group"),
        });

        // --- Shader ---
        // ... (Invariato) ...
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // 6. --- Pipeline ---
        // Ora la pipeline deve conoscere il layout dei nostri uniforms!
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pipeline Layout"),
                // Non più vuoto!
                bind_group_layouts: &[&uniform_bind_group_layout], // <-- MODIFICATO
                push_constant_ranges: &[],
            });

        // ... (La creazione di `render_pipeline` è invariata,
        //      ma ora usa il nuovo `render_pipeline_layout`) ...
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout), // Usa il layout aggiornato
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
        // ... (Invariato) ...
        let initial_size = 6 * 10 * std::mem::size_of::<Vertex>() as u64;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: initial_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

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

            // Campi aggiunti
            uniform_buffer,
            uniform_bind_group,
            uniform_bind_group_layout,
        }
    }

    // --- FUNZIONE RESIZE (MODIFICATA) ---
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        // Aggiorna la configurazione (già presente)
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);

        // 8. Aggiorna il buffer Uniform!
        // Questo invia le nuove dimensioni alla GPU.
        let uniforms = StateUniforms {
            screen_size: [new_size.width as f32, new_size.height as f32],
        };

        self.queue.write_buffer(
            &self.uniform_buffer,
            0, // offset
            bytemuck::bytes_of(&uniforms),
        );
    }

    // --- FUNZIONE UPDATE (INVARIATA) ---
    // Non dobbiamo cambiare nulla qui. `update` leggeva già
    // `self.size` per i calcoli, il che è corretto.
    pub fn update(&mut self) {
        let current_time_secs = self.start_time.elapsed().as_secs_f32();
        let screen_height = self.size.height as f32;
        let screen_width = self.size.width as f32;
        let pixels_per_second = screen_height / FALL_DURATION_SECS;

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
            let c = NOTE_COLOR;

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

    // --- FUNZIONE RENDER (MODIFICATA) ---
    pub fn render(&mut self) {
        let output = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                eprintln!("{:?}", e);
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

            render_pass.set_pipeline(&self.render_pipeline);

            // 9. Colleghiamo il Bind Group!
            // Diciamo alla GPU di usare il nostro pacchetto di uniforms
            // per lo slot `@group(0)`.
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..self.num_vertices, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
