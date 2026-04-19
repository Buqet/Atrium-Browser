use crate::css::value::{ComputedStyle, CssBoxSizing};
use super::geometry::{Rect, EdgeInsets};

pub fn clamp_size(value: f32, min: Option<f32>, max: Option<f32>) -> f32 {
    let mut clamped = value;
    if let Some(min) = min {
        clamped = clamped.max(min);
    }
    if let Some(max) = max {
        clamped = clamped.min(max);
    }
    clamped
}
pub fn compute_content_width(
    style: &ComputedStyle,
    containing_block_width: f32,
    padding: EdgeInsets,
    border: EdgeInsets,
) -> f32 {
    let specified = style.width.unwrap_or(containing_block_width);
    let content = match style.box_sizing {
        CssBoxSizing::BorderBox => (specified - padding.horizontal() - border.horizontal()).max(0.0),
        CssBoxSizing::ContentBox => specified,
    };
    clamp_size(content, style.min_width, style.max_width)
}

pub fn compute_box_width(content_width: f32, padding: EdgeInsets, border: EdgeInsets) -> f32 {
    content_width + padding.horizontal() + border.horizontal()
}

pub fn compute_content_height_if_specified(
    style: &ComputedStyle,
    containing_block_height: f32,
    padding: EdgeInsets,
    border: EdgeInsets,
) -> Option<f32> {
    let specified = style.height?;
    let content = match style.box_sizing {
        CssBoxSizing::BorderBox => (specified - padding.vertical() - border.vertical()).max(0.0),
        CssBoxSizing::ContentBox => specified,
    };
    Some(clamp_size(content, style.min_height, style.max_height))
}