use robinson_css::Color;
use robinson_layout::{Rect, RenderTree, RenderBox, RenderBlockBox};

pub struct Canvas {
    pub render_tree: RenderTree,
    pub width: usize,
    pub height: usize,
}

pub struct SolidColor {
    pub rect: Rect,
    pub color: Color,
}

pub type DisplayList = Vec<SolidColor>;

impl Canvas {
    pub fn new(render_tree: RenderTree, width: usize, height: usize) -> Self {
        Self {
            render_tree,
            width,
            height,
        }
    }

    pub fn get_pixels(&mut self) -> Vec<Color> {
        let white = Color::from_hex("#ffffff");
        let mut pixels = vec![white; self.width * self.height];
        let display_list = build_display_list(&self.render_tree.root);
        for item in display_list {
            self.paint_item(&mut pixels, &item);
        }
        pixels
    }

    fn paint_item(&mut self, pixels: &mut [Color], item: &SolidColor) {
        // Clip the rectangle to the canvas boundaries.
        let x0 = item.rect.x.clamp(0.0, self.width as f32) as usize;
        let y0 = item.rect.y.clamp(0.0, self.height as f32) as usize;
        let x1 = (item.rect.x + item.rect.width).clamp(0.0, self.width as f32) as usize;
        let y1 = (item.rect.y + item.rect.height).clamp(0.0, self.height as f32) as usize;

        for y in y0..y1 {
            for x in x0..x1 {
                // TODO: alpha compositing with existing pixel
                pixels[y * self.width + x] = item.color;
            }
        }
    }
}

pub fn build_display_list(render_box: &RenderBox) -> DisplayList {
    let mut list = Vec::new();
    render_layout_box(&mut list, render_box);
    list
}

fn render_layout_box(list: &mut DisplayList, render_box: &RenderBox) {
    if let RenderBox::Block(block) = render_box {
        make_background(list, block);
        if let Some(color) = block.border_color {
            make_border(list, block, color);
        }
        for child in &block.children {
            render_layout_box(list, child);
        }
    }
}

fn make_background(list: &mut DisplayList, render_block: &RenderBlockBox) {
    if let Some(color) = render_block.background_color {
        list.push(SolidColor {
            color,
            rect: render_block.dimensions.border_box(),
        });
    }
}

fn make_border(list: &mut DisplayList, render_block: &RenderBlockBox, color: Color) {
    let d = &render_block.dimensions;
    let border_box = d.border_box();

    // Left border
    list.push(SolidColor {
        color,
        rect: Rect {
            x: border_box.x,
            y: border_box.y,
            width: d.border.left,
            height: border_box.height,
        },
    });

    // Right border
    list.push(SolidColor {
        color,
        rect: Rect {
            x: border_box.x + border_box.width - d.border.right,
            y: border_box.y,
            width: d.border.right,
            height: border_box.height,
        },
    });

    // Top border
    list.push(SolidColor {
        color,
        rect: Rect {
            x: border_box.x,
            y: border_box.y,
            width: border_box.width,
            height: d.border.top,
        },
    });

    // Bottom border
    list.push(SolidColor {
        color,
        rect: Rect {
            x: border_box.x,
            y: border_box.y + border_box.height - d.border.bottom,
            width: border_box.width,
            height: d.border.bottom,
        },
    });
}
