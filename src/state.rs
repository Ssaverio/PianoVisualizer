// state.rs
use crate::config::*;
use crate::note::{Note, Vertex};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

pub struct State {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: PhysicalSize<u32>,

    pub notes: Vec<Note>,

    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub num_vertices: u32,
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

        let notes = vec![
            Note {
                x: 200.0,
                y: 500.0,
                width: NOTE_WIDTH,
                height: NOTE_HEIGHT,
                color: NOTE_COLOR,
            },
            Note {
                x: 400.0,
                y: 350.0,
                width: NOTE_WIDTH,
                height: NOTE_HEIGHT,
                color: NOTE_COLOR,
            },
            Note {
                x: 600.0,
                y: 700.0,
                width: NOTE_WIDTH,
                height: NOTE_HEIGHT,
                color: NOTE_COLOR,
            },
        ];
        // --- Shader ---
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // --- Pipeline ---
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pipeline Layout"),
                bind_group_layouts: &[],
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
                topology: wgpu::PrimitiveTopology::TriangleList, // adesso ogni rettangolo è indipendente
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let initial_size = 6 * 10 * std::mem::size_of::<Vertex>() as u64; // spazio per 10 note come fallback iniziale
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: initial_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            surface,
            device,
            queue,
            config,
            size,
            notes,
            render_pipeline,
            vertex_buffer,
            num_vertices: 0,
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn update(&mut self) {
        // 1️⃣ Muove tutte le note verso il basso
        for note in &mut self.notes {
            note.y -= NOTE_FALL_SPEED;
            print!("Note_data: {:?}", (note.x, note.y));
        }
        println!(" | Vec_note_len: {}", self.notes.len());

        // 2️⃣ Rimuove le note fuori dallo schermo
        self.notes.retain(|note| note.y > 0.0);

        // 3️⃣ Costruisce i vertici per tutte le note attive
        let mut vertices = Vec::new();
        for note in &self.notes {
            let x = note.x;
            let y = note.y;
            let w = NOTE_WIDTH;
            let h = NOTE_HEIGHT;
            let c = note.color;

            // Due triangoli per disegnare un rettangolo
            vertices.extend_from_slice(&[
                // Triangolo 1
                Vertex {
                    position: [x, y],
                    color: c,
                },
                Vertex {
                    position: [x + w, y],
                    color: c,
                },
                Vertex {
                    position: [x, y - h],
                    color: c,
                },
                // Triangolo 2
                Vertex {
                    position: [x + w, y],
                    color: c,
                },
                Vertex {
                    position: [x + w, y - h],
                    color: c,
                },
                Vertex {
                    position: [x, y - h],
                    color: c,
                },
            ]);
        }

        // 4️⃣ Gestione del buffer GPU (riallocazione dinamica se necessario)
        if !vertices.is_empty() {
            let required_size = (vertices.len() * std::mem::size_of::<Vertex>()) as u64;
            let current_size = self.vertex_buffer.size();

            if required_size > current_size {
                let new_size = required_size * 2; // raddoppio per evitare continue riallocazioni
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

            // Copia i nuovi dati nel buffer GPU
            self.queue
                .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
            self.num_vertices = vertices.len() as u32;
        } else {
            self.num_vertices = 0;
        }
    }

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
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..self.num_vertices, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
