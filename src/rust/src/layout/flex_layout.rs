
use crate::css::value::{ComputedStyle, CssFlexDirection, CssAlignItems, CssJustifyContent};
use super::geometry::{Rect, EdgeInsets};
use super::box_model::{compute_content_width, compute_content_height_if_specified};
use super::{LayoutBox, LayoutContext, BoxType, layout_box};

struct FlexItemInfo {
    index: usize,
    flex_grow: f32,
    flex_shrink: f32,
    flex_basis: f32,
    main_size: f32,
    cross_size: f32,
}

pub fn layout_flex_container(box_: &mut LayoutBox, containing_block: Rect, ctx: &LayoutContext) {
    let style = match box_.style.as_ref() {
        Some(s) => s,
        None => {
            super::block_layout::layout_block(box_, containing_block, ctx);
            return;
        }
    };

    let is_row = matches!(
        style.flex_direction,
        CssFlexDirection::Row | CssFlexDirection::RowReverse
    );
    let is_reverse = matches!(
        style.flex_direction,
        CssFlexDirection::RowReverse | CssFlexDirection::ColumnReverse
    );

    let padding = box_.padding;
    let border = box_.border;
    let gap = style.gap;

    let container_content_width = if is_row {
        compute_content_width(style, containing_block.width, padding, border)
    } else {
        containing_block.width - padding.horizontal() - border.horizontal()
    };
    let container_content_height = if !is_row {
        compute_content_height_if_specified(style, containing_block.height, padding, border)
            .unwrap_or(0.0)
    } else {
        containing_block.height - padding.vertical() - border.vertical()
    };

    box_.rect = Rect::new(
        containing_block.x,
        containing_block.y,
        container_content_width + padding.horizontal() + border.horizontal(),
        container_content_height + padding.vertical() + border.vertical(),
    );

    let mut items: Vec<FlexItemInfo> = Vec::new();
    let mut total_main_basis: f32 = 0.0;
    let mut total_flex_grow: f32 = 0.0;
    let mut total_flex_shrink_scaled: f32 = 0.0;

    let active_indices: Vec<usize> = box_
        .children
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.is_display_none())
        .map(|(i, _)| i)
        .collect();

    if active_indices.is_empty() {
        box_.rect.height = padding.vertical() + border.vertical();
        return;
    }

    for &idx in &active_indices {
        let child = &box_.children[idx];
        let child_style = child.style.as_ref();
        let grow = child_style.map(|s| s.flex_grow).unwrap_or(0.0);
        let shrink = child_style.map(|s| s.flex_shrink).unwrap_or(1.0);
        let basis = child_style.and_then(|s| s.flex_basis).unwrap_or(0.0);

        let cross_size = if is_row {
            child.rect.height
        } else {
            child.rect.width
        };

        items.push(FlexItemInfo {
            index: idx,
            flex_grow: grow,
            flex_shrink: shrink,
            flex_basis: basis,
            main_size: basis,
            cross_size,
        });

        total_main_basis += basis;
        total_flex_grow += grow;
        total_flex_shrink_scaled += shrink * basis.max(0.0);
    }

    let main_available = if is_row {
        container_content_width
    } else {
        container_content_height
    };
    let free_space =
        main_available - total_main_basis - gap * (items.len().max(1) - 1) as f32;

    if free_space > 0.0 && total_flex_grow > 0.0 {
        for item in &mut items {
            item.main_size += free_space * (item.flex_grow / total_flex_grow);
        }
    } else if free_space < 0.0 && total_flex_shrink_scaled > 0.0 {
        for item in &mut items {
            let shrink_factor = (item.flex_shrink * item.flex_basis) / total_flex_shrink_scaled;
            item.main_size = (item.flex_basis + free_space * shrink_factor).max(0.0);
        }
    }

    let padding_main_start = if is_row { padding.left } else { padding.top };
    let border_main_start = if is_row { border.left } else { border.top };
    let container_main_start = if is_row { box_.rect.x } else { box_.rect.y };
    let mut cursor_main = container_main_start + border_main_start + padding_main_start;

    let cross_available = if is_row {
        container_content_height
    } else {
        container_content_width
    };
    let padding_cross_start = if is_row { padding.top } else { padding.left };
    let border_cross_start = if is_row { border.top } else { border.left };
    let container_cross_start = if is_row { box_.rect.y } else { box_.rect.x };

    let mut max_cross_size: f32 = 0.0;
    for item in &items {
        let child = &mut box_.children[item.index];
        let cross_size = if style.align_items == CssAlignItems::Stretch {
            cross_available
        } else {
            item.cross_size
        };
        max_cross_size = max_cross_size.max(cross_size);

        let cross_pos = match style.align_items {
            CssAlignItems::FlexStart => 0.0,
            CssAlignItems::FlexEnd => cross_available - cross_size,
            CssAlignItems::Center => (cross_available - cross_size) / 2.0,
            CssAlignItems::Stretch => 0.0,
            _ => 0.0,
        };

        if is_row {
            child.rect = Rect::new(
                cursor_main,
                container_cross_start + border_cross_start + padding_cross_start + cross_pos,
                item.main_size,
                cross_size,
            );
        } else {
            child.rect = Rect::new(
                container_cross_start + border_cross_start + padding_cross_start + cross_pos,
                cursor_main,
                cross_size,
                item.main_size,
            );
        }
        cursor_main += item.main_size + gap;
    }

    if is_reverse {
        let total_main = cursor_main - gap - (container_main_start + border_main_start + padding_main_start);
        let start = container_main_start + border_main_start + padding_main_start;
        for item in &items {
            let old_main = if is_row {
                box_.children[item.index].rect.x
            } else {
                box_.children[item.index].rect.y
            };
            let new_main = start + total_main - (old_main - start) - item.main_size;
            if is_row {
                box_.children[item.index].rect.x = new_main;
            } else {
                box_.children[item.index].rect.y = new_main;
            }
        }
    }

    let total_cross = max_cross_size
        + border_cross_start * 2.0
        + if is_row {
            padding.vertical()
        } else {
            padding.horizontal()
        };
    if is_row {
        box_.rect.height = total_cross;
        if style.width.is_none() {
            box_.rect.width = (cursor_main - gap - container_main_start) + border_main_start + padding.right;
        }
    } else {
        box_.rect.width = total_cross;
        if style.height.is_none() {
            box_.rect.height = (cursor_main - gap - container_main_start) + border_main_start + padding.bottom;
        }
    }
}