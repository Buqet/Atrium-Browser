use crate::css::value::{ComputedStyle, CssPosition};
use super::geometry::Rect;
use super::{LayoutBox, LayoutContext};

pub fn layout_positioned(box_: &mut LayoutBox, containing_block: Rect, ctx: &LayoutContext) {
    super::block_layout::layout_block(box_, containing_block, ctx);

    let style = match box_.style.as_ref() {
        Some(s) => s,
        None => return,
    };

    let position = &style.position;
    if matches!(position, CssPosition::Static) {
        return;
    }

    let top = style.top;
    let right = style.right;
    let bottom = style.bottom;
    let left = style.left;

    match position {
        CssPosition::Relative => {
            if let Some(v) = top {
                box_.rect.y += v;
            }
            if let Some(v) = left {
                box_.rect.x += v;
            }
            if let Some(v) = bottom {
                box_.rect.y -= v;
            }
            if let Some(v) = right {
                box_.rect.x -= v;
            }
        }
        CssPosition::Absolute | CssPosition::Fixed => {
            let mut new_rect = box_.rect;
            if let Some(v) = top {
                new_rect.y = containing_block.y + v;
            } else if let Some(v) = bottom {
                new_rect.y = containing_block.bottom() - v - new_rect.height;
            }

            if let Some(v) = left {
                new_rect.x = containing_block.x + v;
            } else if let Some(v) = right {
                new_rect.x = containing_block.right() - v - new_rect.width;
            }

            if left.is_some() && right.is_some() {
                new_rect.width = containing_block.right() - right.unwrap() - new_rect.x;
            }
            if top.is_some() && bottom.is_some() {
                new_rect.height = containing_block.bottom() - bottom.unwrap() - new_rect.y;
            }
            box_.rect = new_rect;
        }
        CssPosition::Sticky => {
            if let Some(v) = top {
                box_.rect.y = box_.rect.y.min(containing_block.y + v);
            }
        }
        _ => {}
    }
}
