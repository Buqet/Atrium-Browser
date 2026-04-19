mod geometry;
mod box_model;
mod block_layout;
mod flex_layout;
mod inline_layout;
mod positioned;
mod tree_build;

pub use geometry::{EdgeInsets, Rect, Size};
pub use tree_build::build_layout_tree;

use crate::css::value::ComputedStyle;
use crate::html::HtmlNode;
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoxType {
    Block,
    Inline,
    InlineBlock,
    AnonymousText(String),
    FlexContainer,
    FlexItem,
    GridContainer,
    GridItem,
    Positioned,
    Float,
}

#[derive(Debug, Clone)]
pub struct LayoutBox {
    pub box_type: BoxType,
    pub style: Option<ComputedStyle>,
    pub node_index: usize,
    pub rect: Rect,
    pub margin: EdgeInsets,
    pub padding: EdgeInsets,
    pub border: EdgeInsets,
    pub children: Vec<LayoutBox>,
    pub cached_size: Option<Size>,
    pub url: Option<String>,
    pub image_src: Option<String>,
}

impl LayoutBox {
    pub fn new(box_type: BoxType, style: Option<ComputedStyle>, node_index: usize) -> Self {
        Self {
            box_type,
            style,
            node_index,
            rect: Rect::default(),
            margin: EdgeInsets::default(),
            padding: EdgeInsets::default(),
            border: EdgeInsets::default(),
            children: Vec::new(),
            cached_size: None,
            url: None,
            image_src: None,
        }
    }

    pub fn is_display_none(&self) -> bool {
        self.style
            .as_ref()
            .map(|s| s.display.is_none())
            .unwrap_or(false)
    }

    pub fn is_flex_container(&self) -> bool {
        self.style
            .as_ref()
            .map(|s| s.display.is_flex())
            .unwrap_or(false)
    }

    pub fn content_width(&self) -> f32 {
        self.rect.width - self.padding.horizontal() - self.border.horizontal()
    }

    pub fn content_height(&self) -> f32 {
        self.rect.height - self.padding.vertical() - self.border.vertical()
    }
}

#[derive(Clone, Default)]
pub struct LayoutContext {
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub root_font_size: f32,
    pub default_font_size: f32,
}

impl LayoutContext {
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        Self {
            viewport_width,
            viewport_height,
            root_font_size: 16.0,
            default_font_size: 16.0,
        }
    }
}

pub fn collapse_margins(a: f32, b: f32) -> f32 {
    if a >= 0.0 && b >= 0.0 {
        a.max(b)
    } else if a < 0.0 && b < 0.0 {
        a.min(b)
    } else {
        a + b
    }
}

pub fn layout_box(box_: &mut LayoutBox, containing_block: Rect, ctx: &LayoutContext) {
    match box_.box_type {
        BoxType::Block | BoxType::Float => {
            block_layout::layout_block(box_, containing_block, ctx);
        }
        BoxType::FlexContainer => {
            flex_layout::layout_flex_container(box_, containing_block, ctx);
        }
        BoxType::GridContainer => {
            block_layout::layout_block(box_, containing_block, ctx);
        }
        BoxType::Inline | BoxType::AnonymousText(_) => {
            inline_layout::layout_inline(box_, containing_block, ctx);
        }
        BoxType::InlineBlock => {
            block_layout::layout_block(box_, containing_block, ctx);
        }
        BoxType::Positioned => {
            positioned::layout_positioned(box_, containing_block, ctx);
        }
        _ => {
            block_layout::layout_block(box_, containing_block, ctx);
        }
    }
}

pub fn perform_layout(
    root_node: &HtmlNode,
    styles: &[FxHashMap<String, crate::css::value::CssValue>],
    viewport_width: f32,
    viewport_height: f32,
) -> Vec<(usize, Rect, BoxType)> {
    let mut rects = Vec::new();
    if let Some(mut root_box) =
        build_layout_tree(root_node, styles, viewport_width, viewport_height)
    {
        let cb = Rect::new(0.0, 0.0, viewport_width, viewport_height);
        let ctx = LayoutContext::new(viewport_width, viewport_height);
        layout_box(&mut root_box, cb, &ctx);
        collect_rects(&root_box, &mut rects);
    }
    rects
}

fn collect_rects(box_: &LayoutBox, out: &mut Vec<(usize, Rect, BoxType)>) {
    if !box_.is_display_none() && !box_.rect.is_empty() {
        out.push((box_.node_index, box_.rect, box_.box_type.clone()));
    }
    for child in &box_.children {
        collect_rects(child, out);
    }
}
