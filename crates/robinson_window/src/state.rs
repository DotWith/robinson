use glam::{Mat4, Vec3};
use glium::{
    glutin::dpi::PhysicalSize, implement_vertex, index::NoIndices, uniform, Display, Program,
    Surface, VertexBuffer,
};
use robinson_css::StyleSheet;
use robinson_dom::Node;
use robinson_layout::{Dimensions, Rect, RenderTree};
use robinson_paint::{build_display_list, Canvas, SolidColor};
use robinson_style::StyleTree;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

implement_vertex!(Vertex, position, color);

pub struct State {
    root_node: Node,
    stylesheets: Vec<StyleSheet>,
    display: Display,
    rect_program: Program,
    vertex_buffer: VertexBuffer<Vertex>,
    indices: NoIndices,
}

impl State {
    pub fn new(display: Display, root_node: &Node, stylesheets: &Vec<StyleSheet>) -> Self {
        let (width, height) = display.get_framebuffer_dimensions();
        let canvas = Self::generate_canvas(width as f32, height as f32, root_node, stylesheets);

        let rect_vert = include_str!("rect.vert");
        let rect_frag = include_str!("rect.frag");

        let rect_program = Program::from_source(&display, rect_vert, rect_frag, None).unwrap();

        let vertex_buffer = Self::generate_vertices(&display, canvas);

        let indices = NoIndices(glium::index::PrimitiveType::TrianglesList);

        Self {
            root_node: root_node.clone(),
            stylesheets: stylesheets.clone().to_vec(),
            display,
            rect_program,
            vertex_buffer,
            indices,
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

    fn generate_vertices(display: &Display, canvas: Canvas) -> VertexBuffer<Vertex> {
        let mut vertices = vec![];
        let display_list = build_display_list(&canvas.render_tree.root);
        for item in &display_list {
            paint_item(&mut vertices, item);
        }

        VertexBuffer::new(display, &vertices).unwrap()
    }

    pub fn render(&self) {
        let mut target = self.display.draw();
        target.clear_color(1.0, 1.0, 1.0, 1.0);

        let (w, h) = self.display.get_framebuffer_dimensions();
        let w = w as f32;
        let h = h as f32;
        let yoff = 0.0;

        let box_translate = Mat4::from_translation(Vec3::new(-1.0, yoff / h + 1.0, 0.0));
        let box_scale = Mat4::from_scale(Vec3::new(4.0 / w, -4.0 / h, 1.0));
        let box_trans = (box_translate * box_scale).to_cols_array_2d();
        let uniforms = uniform! { matrix: box_trans };
        target
            .draw(
                &self.vertex_buffer,
                self.indices,
                &self.rect_program,
                &uniforms,
                &Default::default(),
            )
            .unwrap();

        target.finish().unwrap();
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        let canvas = Self::generate_canvas(size.width as f32, size.height as f32, &self.root_node, &self.stylesheets);
        let vertex_buffer = Self::generate_vertices(&self.display, canvas);
        self.vertex_buffer = vertex_buffer;
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
