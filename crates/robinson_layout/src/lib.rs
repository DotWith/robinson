///! Basic CSS block layout.

use robinson_style::{StyledNode, Display};
use robinson_css::Value::{Keyword, Length};
use robinson_css::Unit::Px;
use std::rc::Rc;

// CSS box model. All sizes are in px.

#[derive(Clone, Copy, Default, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Dimensions {
    /// Position of the content area relative to the document origin:
    pub content: Rect,
    // Surrounding edges:
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct EdgeSizes {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

/// A node in the layout tree.
pub struct LayoutBox {
    pub dimensions: Dimensions,
    pub box_type: BoxType,
    pub children: Vec<LayoutBox>,
}

pub enum BoxType {
    BlockNode(Rc<StyledNode>),
    InlineNode(Rc<StyledNode>),
    AnonymousBlock(Rc<StyledNode>),
}

impl LayoutBox {
    fn new(box_type: BoxType) -> LayoutBox {
        LayoutBox {
            box_type: box_type,
            dimensions: Default::default(),
            children: Vec::new(),
        }
    }

    fn get_style_node(&self) -> &Rc<StyledNode> {
        match &self.box_type {
            BoxType::BlockNode(node)
            | BoxType::InlineNode(node)
            | BoxType::AnonymousBlock(node) => &node,
        }
    }
}

/// Transform a style tree into a layout tree.
pub fn layout_tree<'a>(node: &Rc<StyledNode>, containing_block: &mut Dimensions) -> LayoutBox {
    let og_height = containing_block.content.height;

    // The layout algorithm expects the container height to start at 0.
    containing_block.content.height = 0.0;

    let mut root_box = build_layout_tree(node);
    root_box.layout(containing_block);
    containing_block.content.height = og_height;
    root_box
}

/// Build the tree of LayoutBoxes, but don't perform any layout calculations yet.
fn build_layout_tree<'a>(style_node: &Rc<StyledNode>) -> LayoutBox {
    // Create the root box.
    let mut root = LayoutBox::new(match style_node.display() {
        Display::Block => BoxType::BlockNode(Rc::clone(style_node)),
        Display::Inline => BoxType::InlineNode(Rc::clone(style_node)),
        Display::None => panic!("Root node has display: none.")
    });

    // Create the descendant boxes.
    for child in style_node.children.borrow().iter() {
        match child.display() {
            Display::Block => root.children.push(build_layout_tree(child)),
            Display::Inline => root.get_inline_container().children.push(build_layout_tree(child)),
            Display::None => {} // Don't lay out nodes with `display: none;`
        }
    }
    root
}

impl LayoutBox {
    /// Lay out a box and its descendants.
    fn layout(&mut self, containing_block: &mut Dimensions) {
        match self.box_type {
            BoxType::BlockNode(_) => self.layout_block(containing_block),
            BoxType::InlineNode(_) | BoxType::AnonymousBlock(_) => {},
        }
    }

    /// Lay out a block-level element and its descendants.
    fn layout_block(&mut self, containing_block: &mut Dimensions) {
        // Child width can depend on parent width, so we need to calculate this box's width before
        // laying out its children.
        self.calculate_block_width(containing_block);

        // Determine where the box is located within its container.
        self.calculate_block_position(containing_block);

        // Recursively lay out the children of this box.
        self.layout_block_children();

        // Parent height can depend on child height, so `calculate_height` must be called after the
        // children are laid out.
        self.calculate_block_height();
    }

    /// Calculate the width of a block-level non-replaced element in normal flow.
    ///
    /// http://www.w3.org/TR/CSS2/visudet.html#blockwidth
    ///
    /// Sets the horizontal margin/padding/border dimensions, and the `width`.
    fn calculate_block_width(&mut self, containing_block: &mut Dimensions) {
        let style = self.get_style_node();

        // `width` has initial value `auto`.
        let auto = Keyword("auto".to_string());
        let mut width = style.value("width").unwrap_or(auto.clone());

        // margin, border, and padding have initial value 0.
        let zero = Length(0.0, Px);

        let mut margin_left = style.lookup("margin-left", "margin", &zero);
        let mut margin_right = style.lookup("margin-right", "margin", &zero);

        let border_left = style.lookup("border-left-width", "border-width", &zero);
        let border_right = style.lookup("border-right-width", "border-width", &zero);

        let padding_left = style.lookup("padding-left", "padding", &zero);
        let padding_right = style.lookup("padding-right", "padding", &zero);

        let total = sum([&margin_left, &margin_right, &border_left, &border_right,
                         &padding_left, &padding_right, &width].iter().map(|v| v.to_px()));

        // If width is not auto and the total is wider than the container, treat auto margins as 0.
        if width != auto && total > containing_block.content.width {
            if margin_left == auto {
                margin_left = Length(0.0, Px);
            }
            if margin_right == auto {
                margin_right = Length(0.0, Px);
            }
        }

        // Adjust used values so that the above sum equals `containing_block.width`.
        // Each arm of the `match` should increase the total width by exactly `underflow`,
        // and afterward all values should be absolute lengths in px.
        let underflow = containing_block.content.width - total;

        match (width == auto, margin_left == auto, margin_right == auto) {
            // If the values are overconstrained, calculate margin_right.
            (false, false, false) => {
                margin_right = Length(margin_right.to_px() + underflow, Px);
            }

            // If exactly one size is auto, its used value follows from the equality.
            (false, false, true) => { margin_right = Length(underflow, Px); }
            (false, true, false) => { margin_left  = Length(underflow, Px); }

            // If width is set to auto, any other auto values become 0.
            (true, _, _) => {
                if margin_left == auto { margin_left = Length(0.0, Px); }
                if margin_right == auto { margin_right = Length(0.0, Px); }

                if underflow >= 0.0 {
                    // Expand width to fill the underflow.
                    width = Length(underflow, Px);
                } else {
                    // Width can't be negative. Adjust the right margin instead.
                    width = Length(0.0, Px);
                    margin_right = Length(margin_right.to_px() + underflow, Px);
                }
            }

            // If margin-left and margin-right are both auto, their used values are equal.
            (false, true, true) => {
                margin_left = Length(underflow / 2.0, Px);
                margin_right = Length(underflow / 2.0, Px);
            }
        }

        let d = &mut self.dimensions;
        d.content.width = width.to_px();

        d.padding.left = padding_left.to_px();
        d.padding.right = padding_right.to_px();

        d.border.left = border_left.to_px();
        d.border.right = border_right.to_px();

        d.margin.left = margin_left.to_px();
        d.margin.right = margin_right.to_px();
    }

    /// Finish calculating the block's edge sizes, and position it within its containing block.
    ///
    /// http://www.w3.org/TR/CSS2/visudet.html#normal-block
    ///
    /// Sets the vertical margin/padding/border dimensions, and the `x`, `y` values.
    fn calculate_block_position(&mut self, containing_block: &mut Dimensions) {
        let style = self.get_style_node();

        // margin, border, and padding have initial value 0.
        let zero = Length(0.0, Px);

        // If margin-top or margin-bottom is `auto`, the used value is zero.
        let margin = EdgeSizes {
            top: style.lookup("margin-top", "margin", &zero).to_px(),
            bottom: style.lookup("margin-bottom", "margin", &zero).to_px(),
            ..(self.dimensions.margin)
        };

        let border = EdgeSizes {
            top: style.lookup("border-top-width", "border-width", &zero).to_px(),
            bottom: style.lookup("border-bottom-width", "border-width", &zero).to_px(),
            ..(self.dimensions.border)
        };
        let padding = EdgeSizes {
            top: style.lookup("padding-top", "padding", &zero).to_px(),
            bottom: style.lookup("padding-bottom", "padding", &zero).to_px(),
            ..(self.dimensions.padding)
        };

        self.dimensions.margin = margin;
        self.dimensions.border = border;
        self.dimensions.padding = padding;

        let d = &mut self.dimensions;

        d.content.x = containing_block.content.x +
                      d.margin.left + d.border.left + d.padding.left;

        // Position the box below all the previous boxes in the container.
        d.content.y = containing_block.content.height + containing_block.content.y +
                      d.margin.top + d.border.top + d.padding.top;
    }

    /// Lay out the block's children within its content area.
    ///
    /// Sets `self.dimensions.height` to the total content height.
    fn layout_block_children(&mut self) {
        let d = &mut self.dimensions;
        for child in &mut self.children {
            child.layout(d);
            // Increment the height so each child is laid out below the previous one.
            d.content.height = d.content.height + child.dimensions.margin_box().height;
        }
    }

    /// Height of a block-level non-replaced element in normal flow with overflow visible.
    fn calculate_block_height(&mut self) {
        // If the height is set to an explicit length, use that exact length.
        // Otherwise, just keep the value set by `layout_block_children`.
        if let Some(Length(h, Px)) = self.get_style_node().value("height") {
            self.dimensions.content.height = h;
        }
    }

    /// Where a new inline child should go.
    fn get_inline_container(&mut self) -> &mut LayoutBox {
        match &self.box_type {
            BoxType::InlineNode(_) | BoxType::AnonymousBlock(_) => self,
            BoxType::BlockNode(node) => {
                // If we've just generated an anonymous block box, keep using it.
                let last = self.children.last();
                let is_anon = match last {
                    Some(ch) => matches!(ch.box_type, BoxType::AnonymousBlock(_)),
                    _ => false
                };
                if !is_anon {
                    self.children.push(LayoutBox::new(BoxType::AnonymousBlock(Rc::clone(node))))
                }
                self.children.last_mut().unwrap()
            }
        }
    }
}

impl Rect {
    pub fn expanded_by(self, edge: EdgeSizes) -> Rect {
        Rect {
            x: self.x - edge.left,
            y: self.y - edge.top,
            width: self.width + edge.left + edge.right,
            height: self.height + edge.top + edge.bottom,
        }
    }
}

impl Dimensions {
    /// The area covered by the content area plus its padding.
    pub fn padding_box(self) -> Rect {
        self.content.expanded_by(self.padding)
    }
    /// The area covered by the content area plus padding and borders.
    pub fn border_box(self) -> Rect {
        self.padding_box().expanded_by(self.border)
    }
    /// The area covered by the content area plus padding, borders, and margin.
    pub fn margin_box(self) -> Rect {
        self.border_box().expanded_by(self.margin)
    }
}

fn sum<I>(iter: I) -> f32 where I: Iterator<Item=f32> {
    iter.fold(0., |a, b| a + b)
}
