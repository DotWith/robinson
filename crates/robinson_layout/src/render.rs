use std::rc::Rc;

use robinson_css::Color;
use robinson_style::StyleNode;

use crate::{build_layout_tree, Dimensions};

#[derive(Debug)]
pub struct RenderTree {
    pub root: RenderBox,
}

#[derive(Debug)]
pub enum RenderBox {
    Block(RenderBlockBox),
    Inline,
    Anonymous,
}

#[derive(Debug)]
pub struct RenderBlockBox {
    pub dimensions: Dimensions,

    pub color: Option<Color>,
    pub background_color: Option<Color>,
    pub border_color: Option<Color>,
    
    pub children: Vec<RenderBox>,
}

impl RenderTree {
    pub fn new(node: &Rc<StyleNode>, containing_block: &mut Dimensions) -> Self {
        let og_height = containing_block.content.height;
        containing_block.content.height = 0.0;

        let mut bbox = build_layout_tree(node);
        let root = bbox.layout(containing_block);

        containing_block.content.height = og_height;

        Self {
            root,
        }
    }
}
