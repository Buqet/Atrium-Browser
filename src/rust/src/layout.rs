use crate::css::value::{CssValue, ViewportContext, ComputedStyle};
use crate::html::HtmlNode;
use rustc_hash::FxHashMap;




#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }
    pub fn right(&self) -> f32 { self.x + self.width }
    pub fn bottom(&self) -> f32 { self.y + self.height }
    pub fn is_empty(&self) -> bool { self.width <= 0.0 || self.height <= 0.0 }
}


#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}


#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EdgeInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl EdgeInsets {
    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self { top, right, bottom, left }
    }
    pub fn all(value: f32) -> Self {
        Self { top: value, right: value, bottom: value, left: value }
    }
    pub fn horizontal(&self) -> f32 { self.left + self.right }
    pub fn vertical(&self) -> f32 { self.top + self.bottom }
    pub fn is_zero(&self) -> bool {
        self.top == 0.0 && self.right == 0.0 && self.bottom == 0.0 && self.left == 0.0
    }
}




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
    pub url: Option<String>, // For <a> links
    pub image_src: Option<String>, // For <img> tags
}

impl LayoutBox {
    pub fn new(box_type: BoxType, style: Option<ComputedStyle>, node_index: usize) -> Self {
        Self {
            box_type, style, node_index,
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
        self.style.as_ref().map(|s| s.display.is_none()).unwrap_or(false)
    }
    pub fn is_flex_container(&self) -> bool {
        self.style.as_ref().map(|s| s.display.is_flex()).unwrap_or(false)
    }
    pub fn is_grid_container(&self) -> bool {
        self.style.as_ref().map(|s| s.display.is_grid()).unwrap_or(false)
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
        Self { viewport_width, viewport_height, root_font_size: 16.0, default_font_size: 16.0 }
    }
}




fn determine_box_type(node: &HtmlNode, style: &Option<ComputedStyle>) -> BoxType {
    let display = style.as_ref().map(|s| s.display.clone())
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
            if display.is_flex() { BoxType::FlexContainer }
            else if display.is_grid() { BoxType::GridContainer }
            else if display.is_inline_block() { BoxType::InlineBlock }
            else if display.is_inline() { BoxType::Inline }
            else { BoxType::Block }
        }
        _ => BoxType::Block,
    }
}


fn extract_box_model(lb: &mut LayoutBox, _ctx: &LayoutContext) {
    if let Some(ref style) = lb.style {
        lb.margin = EdgeInsets::new(style.margin_top, style.margin_right, style.margin_bottom, style.margin_left);
        lb.padding = EdgeInsets::new(style.padding_top, style.padding_right, style.padding_bottom, style.padding_left);
        lb.border = EdgeInsets::new(style.border_top_width, style.border_right_width, style.border_bottom_width, style.border_left_width);
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
        ComputedStyle::from_style_map(s, &ViewportContext::new(ctx.viewport_width, ctx.viewport_height))
    });

    if let Some(ref s) = style {
        if s.display.is_none() {
            return None;
        }
    }

    let box_type = determine_box_type(node, &style);
    let mut lb = LayoutBox::new(box_type, style, idx);
    extract_box_model(&mut lb, ctx);

    // Extract href from <a> tags
    if let HtmlNode::Element { tag, attributes, .. } = node {
        if tag.to_lowercase() == "a" {
            if let Some(href) = attributes.get("href") {
                lb.url = Some(href.clone());
            }
        }
        // Extract src from <img> tags
        if tag.to_lowercase() == "img" {
            if let Some(src) = attributes.get("src") {
                lb.image_src = Some(src.clone());
            }
        }
    }

    if let HtmlNode::Element { children, .. } = node {
        for child in children {
            if let Some(child_box) = build_recursive(child, styles, index, ctx) {
                let child_box = if lb.is_flex_container() {
                    LayoutBox { box_type: BoxType::FlexItem, ..child_box }
                } else {
                    child_box
                };
                lb.children.push(child_box);
            }
        }
    }

    Some(lb)
}




fn collapse_margins(margin1: f32, margin2: f32) -> f32 {
    if margin1 >= 0.0 && margin2 >= 0.0 {
        margin1.max(margin2)
    } else if margin1 < 0.0 && margin2 < 0.0 {
        -(-margin1).min(-margin2)
    } else {
        margin1 + margin2
    }
}



pub fn layout_block_recursive(
    box_: &mut LayoutBox,
    containing_block: Rect,
    ctx: &LayoutContext,
) {
    if box_.is_display_none() { return; }

    let style = box_.style.as_ref();
    
    let width = style.and_then(|s| s.width).unwrap_or(containing_block.width);
    let total_horizontal = box_.border.horizontal() + box_.padding.horizontal();

    
    let available = containing_block.width - width - total_horizontal;
    let mut margin_left = if box_.margin.left < 0.0 { 0.0 } else { box_.margin.left };
    let mut margin_right = if box_.margin.right < 0.0 { 0.0 } else { box_.margin.right };
    if available > 0.0 {
        let ml_auto = box_.margin.left < 0.0;
        let mr_auto = box_.margin.right < 0.0;
        match (ml_auto, mr_auto) {
            (true, true) => { let half = available / 2.0; margin_left = half; margin_right = half; }
            (true, false) => { margin_left = available - margin_right; }
            (false, true) => { margin_right = available - margin_left; }
            _ => {}
        }
    }

    box_.rect = Rect {
        x: containing_block.x + margin_left,
        y: containing_block.y,
        width,
        height: 0.0,
    };

    let content_x = box_.rect.x + box_.border.left + box_.padding.left;
    let content_width = box_.content_width().max(0.0);
    let content_start_y = box_.rect.y + box_.border.top + box_.padding.top;
    let mut cursor_y = content_start_y;
    let mut prev_margin_bottom: f32 = 0.0;

    for child in box_.children.iter_mut() {
        if child.is_display_none() { continue; }

        
        let child_mt = child.margin.top;
        let collapsed_mt = if prev_margin_bottom > 0.0 && child_mt > 0.0 {
            prev_margin_bottom.max(child_mt)
        } else {
            child_mt
        };

        let child_cb = Rect {
            x: content_x,
            y: cursor_y + collapsed_mt,
            width: content_width,
            height: 0.0,
        };

        layout_box(child, child_cb, ctx);

        
        cursor_y = child.rect.y + child.rect.height;
        prev_margin_bottom = child.margin.bottom;
    }

    
    let content_height = if box_.children.is_empty() {
        0.0
    } else {
        (cursor_y + prev_margin_bottom - content_start_y).max(0.0)
    };
    let final_height = style.and_then(|s| s.height).unwrap_or(content_height);
    box_.rect.height = (final_height + box_.border.vertical() + box_.padding.vertical()).max(0.0);
}



fn layout_box(box_: &mut LayoutBox, containing_block: Rect, ctx: &LayoutContext) {
    match box_.box_type {
        BoxType::Block | BoxType::Float | BoxType::Positioned => {
            layout_positioned_box(box_, containing_block, ctx);
        }
        BoxType::FlexContainer => {
            layout_flex_container(box_, containing_block, ctx);
        }
        BoxType::GridContainer => {
            layout_grid_container(box_, containing_block, ctx);
        }
        BoxType::InlineBlock => {
            
            layout_block_recursive(box_, containing_block, ctx);
        }
        BoxType::Inline | BoxType::AnonymousText(_) => {
            layout_inline_box(box_, containing_block, ctx);
        }
        BoxType::FlexItem => {
            layout_block_recursive(box_, containing_block, ctx);
        }
        _ => {
            layout_block_recursive(box_, containing_block, ctx);
        }
    }
}



fn layout_positioned_box(
    box_: &mut LayoutBox,
    containing_block: Rect,
    ctx: &LayoutContext,
) {
    let position = box_.style.as_ref().map(|s| s.position.clone());
    let top = box_.style.as_ref().and_then(|s| s.top);
    let left = box_.style.as_ref().and_then(|s| s.left);
    let bottom = box_.style.as_ref().and_then(|s| s.bottom);
    let right = box_.style.as_ref().and_then(|s| s.right);

    layout_block_recursive(box_, containing_block, ctx);

    match position {
        Some(crate::css::value::CssPosition::Relative) => {
            if let Some(v) = top { box_.rect.y += v; }
            if let Some(v) = left { box_.rect.x += v; }
            if let Some(v) = bottom { box_.rect.y -= v; }
            if let Some(v) = right { box_.rect.x -= v; }
        }
        Some(crate::css::value::CssPosition::Absolute) | Some(crate::css::value::CssPosition::Fixed) => {
            if let Some(v) = top { box_.rect.y = containing_block.y + v; }
            if let Some(v) = left { box_.rect.x = containing_block.x + v; }
            if let Some(v) = bottom { box_.rect.y = containing_block.bottom() - v - box_.rect.height; }
            if let Some(v) = right { box_.rect.x = containing_block.right() - v - box_.rect.width; }
        }
        Some(crate::css::value::CssPosition::Sticky) => {
            if let Some(v) = top { box_.rect.y = box_.rect.y.min(containing_block.y + v); }
        }
        _ => {}
    }
}



pub fn layout_flex_container(
    box_: &mut LayoutBox,
    containing_block: Rect,
    ctx: &LayoutContext,
) {
    let style = box_.style.as_ref();
    if style.is_none() { layout_block_recursive(box_, containing_block, ctx); return; }
    let style = style.unwrap();

    let flex_direction = style.flex_direction.clone();
    let align_items = style.align_items.clone();
    let gap = style.gap;

    let is_row = matches!(flex_direction, crate::css::value::CssFlexDirection::Row | crate::css::value::CssFlexDirection::RowReverse);
    let is_reverse = matches!(flex_direction, crate::css::value::CssFlexDirection::RowReverse | crate::css::value::CssFlexDirection::ColumnReverse);

    let main_available = if is_row { box_.content_width() } else { box_.content_height().max(0.0) };

    let active_indices: Vec<usize> = box_.children.iter()
        .enumerate().filter(|(_, c)| !c.is_display_none()).map(|(i, _)| i).collect();

    if active_indices.is_empty() {
        box_.rect.height = box_.padding.vertical() + box_.border.vertical();
        return;
    }

    struct FlexItemInfo { index: usize, flex_basis: f32, flex_grow: f32, flex_shrink: f32, main_size: f32 }

    let mut total_main_size: f32 = 0.0;
    let mut total_flex_grow: f32 = 0.0;
    let mut total_flex_shrink: f32 = 0.0;
    let mut items: Vec<FlexItemInfo> = Vec::new();

    for &idx in &active_indices {
        let child = &box_.children[idx];
        let grow = child.style.as_ref().map(|s| s.flex_grow).unwrap_or(0.0);
        let shrink = child.style.as_ref().map(|s| s.flex_shrink).unwrap_or(1.0);
        let basis = child.style.as_ref().and_then(|s| s.flex_basis).unwrap_or(0.0);
        total_flex_grow += grow;
        total_flex_shrink += shrink * basis.max(0.0);
        total_main_size += basis;
        items.push(FlexItemInfo { index: idx, flex_basis: basis, flex_grow: grow, flex_shrink: shrink, main_size: basis });
    }

    let free_space = main_available - total_main_size - gap * (items.len().max(1) - 1) as f32;

    if free_space > 0.0 && total_flex_grow > 0.0 {
        for item in &mut items { item.main_size += free_space * (item.flex_grow / total_flex_grow); }
    } else if free_space < 0.0 && total_flex_shrink > 0.0 {
        for item in &mut items {
            let shrink_factor = item.flex_shrink * item.flex_basis / total_flex_shrink;
            item.main_size = (item.flex_basis + free_space * shrink_factor).max(0.0);
        }
    }

    let padding_main_start = if is_row { box_.padding.left } else { box_.padding.top };
    let padding_cross_start = if is_row { box_.padding.top } else { box_.padding.left };
    let border_main_start = if is_row { box_.border.left } else { box_.border.top };
    let border_cross_start = if is_row { box_.border.top } else { box_.border.left };
    let container_main_start = if is_row { box_.rect.x } else { box_.rect.y };
    let container_cross_start = if is_row { box_.rect.y } else { box_.rect.x };
    let container_cross_size = if is_row { box_.rect.height } else { box_.rect.width };
    let cross_available = container_cross_size - box_.padding.vertical() - box_.border.vertical();

    let mut cursor_main = container_main_start + border_main_start + padding_main_start;
    let mut max_cross_size: f32 = 0.0;

    for item in &items {
        let child = &box_.children[item.index];
        let cross_size = if is_row { child.rect.height.max(0.0) } else { child.rect.width.max(0.0) };
        max_cross_size = max_cross_size.max(cross_size);

        let cross_pos = match align_items {
            crate::css::value::CssAlignItems::FlexStart => padding_cross_start,
            crate::css::value::CssAlignItems::FlexEnd => cross_available - cross_size - padding_cross_start,
            crate::css::value::CssAlignItems::Center => (cross_available - cross_size) / 2.0,
            _ => padding_cross_start,
        };

        if is_row {
            box_.children[item.index].rect = Rect::new(
                cursor_main, container_cross_start + border_cross_start + cross_pos,
                item.main_size, child.rect.height.max(cross_size));
        } else {
            box_.children[item.index].rect = Rect::new(
                container_cross_start + border_cross_start + cross_pos, cursor_main,
                child.rect.width.max(cross_size), item.main_size);
        }
        cursor_main += item.main_size + gap;
    }

    if is_reverse {
        let total_main = cursor_main - gap - (container_main_start + border_main_start + padding_main_start);
        let start = container_main_start + border_main_start + padding_main_start;
        for item in &items {
            let old_main = if is_row { box_.children[item.index].rect.x } else { box_.children[item.index].rect.y };
            let new_main = start + total_main - (old_main - start) - item.main_size;
            if is_row { box_.children[item.index].rect.x = new_main; } else { box_.children[item.index].rect.y = new_main; }
        }
    }

    let total_main_used = cursor_main - gap + border_main_start * 2.0 + box_.padding.horizontal();
    let total_cross_used = max_cross_size + border_cross_start * 2.0 + box_.padding.vertical();
    if is_row { box_.rect.width = total_main_used.max(box_.rect.width); box_.rect.height = total_cross_used; }
    else { box_.rect.width = total_cross_used; box_.rect.height = total_main_used.max(box_.rect.height); }
}



fn layout_grid_container(box_: &mut LayoutBox, containing_block: Rect, ctx: &LayoutContext) {
    layout_block_recursive(box_, containing_block, ctx);
}



fn place_floats(box_: &mut LayoutBox, containing_block: Rect, ctx: &LayoutContext) {
    layout_block_recursive(box_, containing_block, ctx);
}



#[derive(Debug, Clone)]
struct InlineChild { text: String, width: f32, height: f32, font_size: f32, x: f32, y: f32 }

pub fn layout_inline_box(box_: &mut LayoutBox, containing_block: Rect, ctx: &LayoutContext) {
    let text = match &box_.box_type {
        BoxType::AnonymousText(t) => t.clone(),
        _ => { box_.rect = containing_block; box_.rect.height = 20.0; return; }
    };

    let font_size = box_.style.as_ref().map(|s| s.font_size).unwrap_or(ctx.default_font_size);
    let line_height = box_.style.as_ref().and_then(|s| s.line_height).unwrap_or(font_size * 1.2);
    let text_align = box_.style.as_ref().map(|s| s.text_align.clone()).unwrap_or(crate::css::value::CssTextAlign::Left);

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() { box_.rect = Rect::new(containing_block.x, containing_block.y, 0.0, 0.0); return; }

    let avg_char_width = font_size * 0.5;
    let space_width = font_size * 0.25;
    let measure_word_width = |word: &str| -> f32 { word.len() as f32 * avg_char_width };
    let available_width = containing_block.width;

    let mut lines: Vec<Vec<(&str, f32)>> = Vec::new();
    let mut current_line: Vec<(&str, f32)> = Vec::new();
    let mut current_width: f32 = 0.0;

    for word in &words {
        let word_width = measure_word_width(word);
        if !current_line.is_empty() && current_width + space_width + word_width > available_width {
            lines.push(std::mem::take(&mut current_line));
            current_width = 0.0;
        }
        current_line.push((word, word_width));
        current_width += if current_line.len() > 1 { space_width + word_width } else { word_width };
    }
    if !current_line.is_empty() { lines.push(current_line); }

    let mut cursor_y = containing_block.y;
    for line in &lines {
        let line_width: f32 = line.iter().map(|(_, w)| w).sum::<f32>() + (line.len() as f32 - 1.0) * space_width;
        let mut cursor_x = containing_block.x;
        match text_align {
            crate::css::value::CssTextAlign::Center => { cursor_x += (available_width - line_width) / 2.0; }
            crate::css::value::CssTextAlign::Right => { cursor_x += available_width - line_width; }
            _ => {}
        }
        for (word, word_width) in line {
            let mut child_box = LayoutBox::new(
                BoxType::AnonymousText(word.to_string()), box_.style.clone(), box_.node_index);
            child_box.rect = Rect::new(cursor_x, cursor_y, *word_width, line_height);
            box_.children.push(child_box);
            cursor_x += word_width + space_width;
        }
        cursor_y += line_height;
    }
    box_.rect = Rect::new(containing_block.x, containing_block.y, available_width, cursor_y - containing_block.y);
}





pub fn collect_layout_rects(root: &LayoutBox) -> Vec<(usize, Rect, BoxType)> {
    let mut rects = Vec::new();
    collect_rects_recursive(root, &mut rects);
    rects
}

fn collect_rects_recursive(box_: &LayoutBox, rects: &mut Vec<(usize, Rect, BoxType)>) {
    if !box_.is_display_none() && !box_.rect.is_empty() {
        rects.push((box_.node_index, box_.rect, box_.box_type.clone()));
    }
    for child in &box_.children {
        collect_rects_recursive(child, rects);
    }
}


pub fn perform_layout(
    root_node: &HtmlNode,
    styles: &[FxHashMap<String, CssValue>],
    viewport_width: f32,
    viewport_height: f32,
) -> Vec<(usize, Rect, BoxType)> {
    let mut rects = Vec::new();
    if let Some(mut root_box) = build_layout_tree(root_node, styles, viewport_width, viewport_height) {
        let cb = Rect::new(0.0, 0.0, viewport_width, viewport_height);
        let ctx = LayoutContext::new(viewport_width, viewport_height);
        layout_box(&mut root_box, cb, &ctx);
        collect_rects_recursive(&root_box, &mut rects);
    }
    rects
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_basics() {
        let r = Rect::new(10.0, 20.0, 100.0, 200.0);
        assert_eq!(r.right(), 110.0);
        assert_eq!(r.bottom(), 220.0);
        assert!(!r.is_empty());
        assert!(Rect::new(0.0, 0.0, 0.0, 0.0).is_empty());
    }

    #[test]
    fn test_edge_basics() {
        let e = EdgeInsets::new(10.0, 20.0, 30.0, 40.0);
        assert_eq!(e.horizontal(), 60.0);
        assert_eq!(e.vertical(), 40.0);
        assert!(!e.is_zero());
        assert!(EdgeInsets::all(0.0).is_zero());
    }

    #[test]
    fn test_collapse_margins() {
        assert_eq!(collapse_margins(20.0, 30.0), 30.0);
        assert_eq!(collapse_margins(20.0, 20.0), 20.0);
        assert_eq!(collapse_margins(-20.0, -30.0), -20.0);
        assert_eq!(collapse_margins(20.0, -10.0), 10.0);
    }

    #[test]
    fn test_build_layout_tree_simple() {
        use crate::html::HtmlParser;
        let html = "<html><body><div>Test</div></body></html>";
        let mut parser = HtmlParser::new();
        let nodes = parser.parse(html).unwrap();
        let styles: Vec<FxHashMap<String, CssValue>> = vec![FxHashMap::default(); nodes.len()];

        let root = build_layout_tree(&nodes[0], &styles, 1920.0, 1080.0);
        assert!(root.is_some());
        let root = root.unwrap();
        assert!(!root.children.is_empty());
    }

    #[test]
    fn test_layout_box_creation() {
        let lb = LayoutBox::new(BoxType::Block, None, 0);
        assert!(!lb.is_display_none());
        assert!(!lb.is_flex_container());
        assert_eq!(lb.content_width(), 0.0);
    }

    #[test]
    fn test_collect_layout_rects() {
        let mut root = LayoutBox::new(BoxType::Block, None, 0);
        root.rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut child = LayoutBox::new(BoxType::Block, None, 1);
        child.rect = Rect::new(10.0, 10.0, 50.0, 50.0);
        root.children.push(child);

        let rects = collect_layout_rects(&root);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].0, 0);
        assert_eq!(rects[1].0, 1);
    }

    #[test]
    fn test_perform_layout() {
        use crate::html::HtmlParser;
        let html = "<html><body><div style=\"width:100px;height:50px\">Test</div></body></html>";
        let mut parser = HtmlParser::new();
        let nodes = parser.parse(html).unwrap();
        let styles: Vec<FxHashMap<String, CssValue>> = vec![FxHashMap::default(); nodes.len()];

        let rects = perform_layout(&nodes[0], &styles, 800.0, 600.0);
        assert!(!rects.is_empty());
    }
}
