
use crate::html::HtmlNode;
use crate::css::value::{CssValue, ComputedStyle, ViewportContext};
use rustc_hash::FxHashMap;
use super::{LayoutBox, BoxType, EdgeInsets, LayoutContext};

fn extract_box_model(lb: &mut LayoutBox, _ctx: &LayoutContext) {
    if let Some(ref style) = lb.style {
        lb.margin = EdgeInsets::new(
            style.margin_top,
            style.margin_right,
            style.margin_bottom,
            style.margin_left,
        );
        lb.padding = EdgeInsets::new(
            style.padding_top,
            style.padding_right,
            style.padding_bottom,
            style.padding_left,
        );
        lb.border = EdgeInsets::new(
            style.border_top_width,
            style.border_right_width,
            style.border_bottom_width,
            style.border_left_width,
        );
    }
}

pub fn build_layout_tree(
    node: &HtmlNode,
    styles: &[FxHashMap<String, CssValue>],
    viewport_width: f32,
    viewport_height: f32,
) -> Option<LayoutBox> {
    let ctx = LayoutContext::new(viewport_width, viewport_height);
    let mut index = 0;
    build_recursive(node, styles, &mut index, &ctx)
}

fn build_recursive(
    node: &HtmlNode,
    styles: &[FxHashMap<String, CssValue>],
    index: &mut usize,
    ctx: &LayoutContext,
) -> Option<LayoutBox> {
    let idx = *index;
    *index += 1;

    let style = styles.get(idx).map(|s| {
        ComputedStyle::from_style_map(
            s,
            &ViewportContext::new(ctx.viewport_width, ctx.viewport_height),
        )
    });

    if let Some(ref s) = style {
        if s.display.is_none() {
            return None;
        }
    }

    let box_type = determine_box_type(node, &style);
    let mut lb = LayoutBox::new(box_type, style, idx);
    extract_box_model(&mut lb, ctx);

    if let HtmlNode::Element {
        tag, attributes, ..
    } = node
    {
        if tag.to_lowercase() == "a" {
            lb.url = attributes.get("href").cloned();
        } else if tag.to_lowercase() == "img" {
            lb.image_src = attributes.get("src").cloned();
        }
    }

    if let HtmlNode::Element { children, .. } = node {
        for child in children {
            if let Some(child_box) = build_recursive(child, styles, index, ctx) {
                let child_box = if lb.is_flex_container() {
                    LayoutBox {
                        box_type: BoxType::FlexItem,
                        ..child_box
                    }
                } else {
                    child_box
                };
                lb.children.push(child_box);
            }
        }
    }

    Some(lb)
}

pub fn determine_box_type(node: &HtmlNode, style: &Option<ComputedStyle>) -> BoxType {
    let display = style
        .as_ref()
        .map(|s| s.display.clone())
        .unwrap_or(crate::css::value::CssDisplay::Inline);

    match node {
        HtmlNode::Text(text) => {
            if text.trim().is_empty() {
                BoxType::AnonymousText(String::new())
            } else {
                BoxType::AnonymousText(text.clone())
            }
        }
        HtmlNode::Element { .. } => {
            if display.is_flex() {
                BoxType::FlexContainer
            } else if display.is_grid() {
                BoxType::GridContainer
            } else if display.is_inline_block() {
                BoxType::InlineBlock
            } else if display.is_inline() {
                BoxType::Inline
            } else {
                BoxType::Block
            }
        }
        _ => BoxType::Block,
    }
}