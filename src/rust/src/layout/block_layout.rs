use crate::css::value::{ComputedStyle, CssPosition, CssDisplay};
use super::geometry::{Rect, EdgeInsets};
use super::box_model::{compute_content_width, compute_content_height_if_specified, compute_box_width};
use super::{LayoutBox, LayoutContext, BoxType, collapse_margins};

pub fn layout_block(box_: &mut LayoutBox, containing_block: Rect, ctx: &LayoutContext) {
    if box_.is_display_none() {
        return;
    }

    let style = box_.style.as_ref();
    let padding = box_.padding;
    let border = box_.border;
    let margin = box_.margin;

    let content_width = if let Some(s) = style {
        compute_content_width(s, containing_block.width, padding, border)
    } else {
        containing_block.width - padding.horizontal() - border.horizontal()
    };
    let box_width = compute_box_width(content_width, padding, border);

    let available = containing_block.width - box_width;
    let mut margin_left = margin.left;
    let mut margin_right = margin.right;
    if available > 0.0 {
        let ml_auto = margin.left < 0.0; // we use negative as marker for auto? Better to have explicit bool. For now keep as is.
        let mr_auto = margin.right < 0.0;
        match (ml_auto, mr_auto) {
            (true, true) => {
                let half = available / 2.0;
                margin_left = half;
                margin_right = half;
            }
            (true, false) => margin_left = available - margin_right,
            (false, true) => margin_right = available - margin_left,
            _ => {}
        }
    }

    box_.rect.x = containing_block.x + margin_left;
    box_.rect.y = containing_block.y;
    box_.rect.width = box_width;

    let content_x = box_.rect.x + border.left + padding.left;
    let content_start_y = box_.rect.y + border.top + padding.top;
    let content_width_avail = content_width.max(0.0);
    let mut cursor_y = content_start_y;

    let mut prev_margin_bottom = 0.0;
    let is_first_child = true;

    for child in box_.children.iter_mut() {
        if child.is_display_none() {
            continue;
        }

        let child_mt = child.margin.top;
        let collapsed_mt = if !is_first_child {
            collapse_margins(prev_margin_bottom, child_mt)
        } else {
            child_mt
        };

        let effective_mt = if is_first_child && border.top == 0.0 && padding.top == 0.0 {
            collapse_margins(box_.margin.top, collapsed_mt)
        } else {
            collapsed_mt
        };

        let child_cb = Rect {
            x: content_x,
            y: cursor_y + effective_mt,
            width: content_width_avail,
            height: 0.0,
        };

        layout_box(child, child_cb, ctx);

        cursor_y = child.rect.y + child.rect.height;
        prev_margin_bottom = child.margin.bottom;
        is_first_child = false;
    }

    let content_height = if box_.children.is_empty() {
        0.0
    } else {
        (cursor_y + prev_margin_bottom - content_start_y).max(0.0)
    };

    let final_height = if let Some(s) = style {
        compute_content_height_if_specified(s, containing_block.height, padding, border)
            .map(|h| h + padding.vertical() + border.vertical())
            .unwrap_or(content_height + padding.vertical() + border.vertical())
    } else {
        content_height + padding.vertical() + border.vertical()
    };

    box_.rect.height = final_height;

    if !box_.children.is_empty() && border.bottom == 0.0 && padding.bottom == 0.0 {
        let last_mb = box_.children.last().unwrap().margin.bottom;
        box_.margin.bottom = collapse_margins(box_.margin.bottom, last_mb);
    }
}
