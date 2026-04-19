use crate::css::value::{ComputedStyle, CssTextAlign, CssVerticalAlign};
use super::geometry::Rect;
use super::{LayoutBox, LayoutContext, BoxType};

pub fn layout_inline(box_: &mut LayoutBox, containing_block: Rect, ctx: &LayoutContext) {
    let text = match &box_.box_type {
        BoxType::AnonymousText(t) => t.clone(),
        _ => {
            box_.rect = containing_block;
            box_.rect.height = 20.0;
            return;
        }
    };

    let style = box_.style.as_ref();
    let font_size = style.map(|s| s.font_size).unwrap_or(ctx.default_font_size);
    let line_height = style
        .and_then(|s| s.line_height)
        .unwrap_or(font_size * 1.2);
    let text_align = style
        .map(|s| s.text_align.clone())
        .unwrap_or(CssTextAlign::Left);
    let vertical_align = style
        .map(|s| s.vertical_align.clone())
        .unwrap_or(CssVerticalAlign::Baseline);

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        box_.rect = Rect::new(containing_block.x, containing_block.y, 0.0, 0.0);
        return;
    }

    let avg_char_width = font_size * 0.5;
    let space_width = font_size * 0.25;
    let measure_word = |w: &str| w.len() as f32 * avg_char_width;
    let available_width = containing_block.width;

    let mut lines: Vec<Vec<(&str, f32)>> = Vec::new();
    let mut current_line: Vec<(&str, f32)> = Vec::new();
    let mut current_width = 0.0;

    for word in &words {
        let w = measure_word(word);
        if !current_line.is_empty() && current_width + space_width + w > available_width {
            lines.push(std::mem::take(&mut current_line));
            current_width = 0.0;
        }
        current_line.push((word, w));
        current_width += if current_line.len() > 1 {
            space_width + w
        } else {
            w
        };
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    let mut cursor_y = containing_block.y;
    for line in &lines {
        let line_width: f32 = line.iter().map(|(_, w)| w).sum::<f32>()
            + (line.len() - 1) as f32 * space_width;
        let mut cursor_x = containing_block.x;
        match text_align {
            CssTextAlign::Center => cursor_x += (available_width - line_width) / 2.0,
            CssTextAlign::Right => cursor_x += available_width - line_width,
            _ => {}
        }

        let line_box_height = line_height;
        let max_ascent = font_size * 0.8;
        for (word, word_width) in line {
            let mut child_box = LayoutBox::new(
                BoxType::AnonymousText(word.to_string()),
                box_.style.clone(),
                box_.node_index,
            );
            let y_offset = match vertical_align {
                CssVerticalAlign::Middle => (line_box_height - font_size) / 2.0,
                CssVerticalAlign::Top => 0.0,
                CssVerticalAlign::Bottom => line_box_height - font_size,
                _ => max_ascent,
            };
            child_box.rect = Rect::new(cursor_x, cursor_y + y_offset, *word_width, font_size);
            box_.children.push(child_box);
            cursor_x += word_width + space_width;
        }
        cursor_y += line_height;
    }

    box_.rect = Rect::new(
        containing_block.x,
        containing_block.y,
        available_width,
        cursor_y - containing_block.y,
    );
}