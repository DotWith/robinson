use std::{fs::File, io::{BufWriter, self}};

use glam::{Mat4, Vec3};
use robinson_css::StyleSheet;
use robinson_dom::Node;
use robinson_layout::{Dimensions, Rect, RenderTree};
use robinson_paint::{build_display_list, Canvas, SolidColor};
use robinson_style::StyleTree;
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, window::Window};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub struct State {
    root_node: Node,
    stylesheets: Vec<StyleSheet>,
    window_size: PhysicalSize<u32>,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
    camera_uniform: [[f32; 4]; 4],
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
}

impl State {
    pub async fn new(window: &Window, root_node: &Node, stylesheets: &Vec<StyleSheet>) -> Self {
        let window_size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .enumerate_adapters(wgpu::Backends::all())
            .filter(|adapter| adapter.is_surface_supported(&surface))
            .next()
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window_size.width,
            height: window_size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let rect_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rect Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("rect.wgsl").into()),
        });

        let camera_uniform = Self::generate_matrix(window_size);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
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
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &rect_shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &rect_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let canvas = Self::generate_canvas(
            window_size.width as f32,
            window_size.height as f32,
            root_node,
            stylesheets,
        );

        let vertices = Self::generate_vertices(canvas);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let num_vertices = vertices.len() as u32;

        Self {
            root_node: root_node.clone(),
            stylesheets: stylesheets.clone().to_vec(),
            window_size,
            surface,
            device,
            queue,
            render_pipeline,
            vertex_buffer,
            num_vertices,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
        }
    }

    fn generate_canvas(
        width: f32,
        height: f32,
        root_node: &Node,
        stylesheets: &Vec<StyleSheet>,
    ) -> Canvas {
        let mut viewport = Dimensions {
            content: Rect {
                width: width / 2.0,
                height: height / 2.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let style_tree = StyleTree::new(root_node, stylesheets);
        let render_tree = RenderTree::new(&style_tree.root.borrow(), &mut viewport);

        Canvas::new(
            render_tree,
            viewport.content.width as usize,
            viewport.content.height as usize,
        )
    }

    fn generate_vertices(canvas: Canvas) -> Vec<Vertex> {
        let mut vertices = vec![];
        let display_list = build_display_list(&canvas.render_tree.root);
        for item in &display_list {
            paint_item(&mut vertices, item);
        }

        vertices
    }

    fn generate_matrix(size: PhysicalSize<u32>) -> [[f32; 4]; 4] {
        let w = size.width as f32;
        let h = size.height as f32;
        let yoff = 0.0;

        let box_translate = Mat4::from_translation(Vec3::new(-1.0, yoff / h + 1.0, 0.0));
        let box_scale = Mat4::from_scale(Vec3::new(4.0 / w, -4.0 / h, 1.0));
        (box_translate * box_scale).to_cols_array_2d()
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
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
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..self.num_vertices, 0..1);
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.window_size = size;

        let canvas = Self::generate_canvas(
            size.width as f32,
            size.height as f32,
            &self.root_node,
            &self.stylesheets,
        );
        let verts = Self::generate_vertices(canvas);
        self.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&verts));

        self.camera_uniform = Self::generate_matrix(size);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    pub fn print_pdf(&self) -> Result<(), io::Error> {
        let canvas = Self::generate_canvas(
            self.window_size.width as f32,
            self.window_size.height as f32,
            &self.root_node,
            &self.stylesheets,
        );
        let mut file = BufWriter::new(File::create(&"output.pdf").unwrap());
        robinson_pdf::render(
            &canvas.render_tree,
            canvas.width as f32,
            canvas.height as f32,
            &mut file,
        )?;
        Ok(())
    }
}

fn paint_item(vertices: &mut Vec<Vertex>, item: &SolidColor) {
    let x0 = item.rect.x;
    let y0 = item.rect.y;
    let x1 = item.rect.x + item.rect.width;
    let y1 = item.rect.y + item.rect.height;

    let color = [
        item.color.r as f32 / 255.0,
        item.color.g as f32 / 255.0,
        item.color.b as f32 / 255.0,
        1.0,
    ];

    // Triangle 1
    vertices.push(Vertex {
        position: [x0, y0],
        color,
    });
    vertices.push(Vertex {
        position: [x0, y1],
        color,
    });
    vertices.push(Vertex {
        position: [x1, y1],
        color,
    });

    // Triangle 2
    vertices.push(Vertex {
        position: [x1, y1],
        color,
    });
    vertices.push(Vertex {
        position: [x1, y0],
        color,
    });
    vertices.push(Vertex {
        position: [x0, y0],
        color,
    });
}
