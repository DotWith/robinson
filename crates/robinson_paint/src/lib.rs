use robinson_layout::{BoxType, LayoutBox, Rect};
use robinson_css::{Value, Color};

pub struct Canvas {
    pub layout: LayoutBox,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug)]
pub enum DisplayCommand {
    SolidColor(Color, Rect),
}

pub type DisplayList = Vec<DisplayCommand>;

impl Canvas {
    pub fn new(layout: LayoutBox, width: usize, height: usize) -> Self {
        Self { layout, width, height }
    }

    pub fn get_pixels(&mut self) -> Vec<Color> {
        let white = Color::from_hex("#ffffff");
        let mut pixels = vec![white; self.width * self.height];
        let display_list = build_display_list(&self.layout);
        for item in display_list {
            self.paint_item(&mut pixels, &item);
        }
        pixels
    }

    fn paint_item(&mut self, pixels: &mut [Color], item: &DisplayCommand) {
        match *item {
            DisplayCommand::SolidColor(color, rect) => {
                // Clip the rectangle to the canvas boundaries.
                let x0 = rect.x.clamp(0.0, self.width as f32) as usize;
                let y0 = rect.y.clamp(0.0, self.height as f32) as usize;
                let x1 = (rect.x + rect.width).clamp(0.0, self.width as f32) as usize;
                let y1 = (rect.y + rect.height).clamp(0.0, self.height as f32) as usize;

                for y in y0 .. y1 {
                    for x in x0 .. x1 {
                        // TODO: alpha compositing with existing pixel
                        pixels[y * self.width + x] = color;
                    }
                }
            }
        }
    }
}

pub fn build_display_list(layout_root: &LayoutBox) -> DisplayList {
    let mut list = Vec::new();
    render_layout_box(&mut list, layout_root);
    list
}

fn render_layout_box(list: &mut DisplayList, layout_box: &LayoutBox) {
    render_background(list, layout_box);
    render_borders(list, layout_box);
    for child in &layout_box.children {
        render_layout_box(list, child);
    }
}

fn render_background(list: &mut DisplayList, layout_box: &LayoutBox) {
    if let Some(color) = get_color(layout_box, "background") {
        list.push(DisplayCommand::SolidColor(color, layout_box.dimensions.border_box()));
    }
}

fn render_borders(list: &mut DisplayList, layout_box: &LayoutBox) {
    let color = match get_color(layout_box, "border-color") {
        Some(color) => color,
        _ => return
    };

    let d = &layout_box.dimensions;
    let border_box = d.border_box();

    // Left border
    list.push(DisplayCommand::SolidColor(color, Rect {
        x: border_box.x,
        y: border_box.y,
        width: d.border.left,
        height: border_box.height,
    }));

    // Right border
    list.push(DisplayCommand::SolidColor(color, Rect {
        x: border_box.x + border_box.width - d.border.right,
        y: border_box.y,
        width: d.border.right,
        height: border_box.height,
    }));

    // Top border
    list.push(DisplayCommand::SolidColor(color, Rect {
        x: border_box.x,
        y: border_box.y,
        width: border_box.width,
        height: d.border.top,
    }));

    // Bottom border
    list.push(DisplayCommand::SolidColor(color, Rect {
        x: border_box.x,
        y: border_box.y + border_box.height - d.border.bottom,
        width: border_box.width,
        height: d.border.bottom,
    }));
}

/// Return the specified color for CSS property `name`, or None if no color was specified.
fn get_color(layout_box: &LayoutBox, name: &str) -> Option<Color> {
    match &layout_box.box_type {
        BoxType::BlockNode(style)
        | BoxType::InlineNode(style)
        | BoxType::AnonymousBlock(style) => match style.get_value(name) {
            Some(Value::Color(color)) => Some(color),
            _ => None
        },
    }
}
