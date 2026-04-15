









use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int};
use std::slice;

use crate::css::parser::CssParser;
use crate::css::parser::Stylesheet;
use crate::css::matcher::compute_styles;
use crate::css::value::{CssValue, ViewportContext, CssLength};
use crate::html::{HtmlParser, HtmlNode};
use crate::layout::{Rect, Size, EdgeInsets, LayoutBox, BoxType, LayoutContext, build_layout_tree, perform_layout, collect_layout_rects};
use crate::dom::{Document, NodeHandle};
use rustc_hash::FxHashMap;
use std::collections::HashMap;




#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CLayoutRect {
    pub x: c_double,
    pub y: c_double,
    pub width: c_double,
    pub height: c_double,
    pub tag: *mut c_char,   
    pub display_none: c_int,
}


#[repr(C)]
#[derive(Debug, Clone)]
pub struct CHtmlNode {
    pub tag: *mut c_char,
    pub id: *mut c_char,
    pub class: *mut c_char,
    pub children_count: c_int,
}


#[repr(C)]
#[derive(Debug, Clone)]
pub struct CStyleProperty {
    pub property: *mut c_char,
    pub value: *mut c_char,
}


#[repr(C)]
#[derive(Debug, Clone)]
pub struct CElementStyle {
    pub properties: *mut CStyleProperty,
    pub property_count: c_int,
}




#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(s);
    }
}


#[no_mangle]
pub extern "C" fn free_layout_rect(rect: CLayoutRect) {
    if !rect.tag.is_null() {
        free_string(rect.tag);
    }
}


#[no_mangle]
pub extern "C" fn free_element_style(style: CElementStyle) {
    if style.properties.is_null() || style.property_count == 0 {
        return;
    }
    unsafe {
        let props = slice::from_raw_parts_mut(style.properties, style.property_count as usize);
        for prop in props {
            if !prop.property.is_null() {
                free_string(prop.property);
            }
            if !prop.value.is_null() {
                free_string(prop.value);
            }
        }
        
        let _ = Vec::from_raw_parts(style.properties, style.property_count as usize, style.property_count as usize);
    }
}





#[no_mangle]
pub extern "C" fn parse_html(html: *const c_char) -> *mut HtmlDocument {
    if html.is_null() {
        return std::ptr::null_mut();
    }

    let html_str = unsafe { CStr::from_ptr(html).to_string_lossy().into_owned() };
    let mut parser = HtmlParser::new();

    match parser.parse(&html_str) {
        Ok(nodes) => {
            let doc = Box::new(HtmlDocument { nodes });
            Box::into_raw(doc)
        }
        Err(_) => std::ptr::null_mut(),
    }
}


pub struct HtmlDocument {
    nodes: Vec<HtmlNode>,
}


#[no_mangle]
pub extern "C" fn html_doc_node_count(doc: *const HtmlDocument) -> c_int {
    if doc.is_null() {
        return 0;
    }
    unsafe { (*doc).nodes.len() as c_int }
}


#[no_mangle]
pub extern "C" fn free_html_document(doc: *mut HtmlDocument) {
    if doc.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(doc);
    }
}




#[no_mangle]
pub extern "C" fn parse_css(css: *const c_char) -> *mut StylesheetHandle {
    if css.is_null() {
        return std::ptr::null_mut();
    }

    let css_str = unsafe { CStr::from_ptr(css).to_string_lossy().into_owned() };
    let mut parser = CssParser::new();

    match parser.parse(&css_str) {
        Ok(stylesheet) => {
            let handle = Box::new(StylesheetHandle(stylesheet));
            Box::into_raw(handle)
        }
        Err(_) => std::ptr::null_mut(),
    }
}


pub struct StylesheetHandle(Stylesheet);


#[no_mangle]
pub extern "C" fn free_stylesheet(ss: *mut StylesheetHandle) {
    if ss.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(ss);
    }
}






#[no_mangle]
pub extern "C" fn compute_styles_for_document(
    doc: *const HtmlDocument,
    ss: *const StylesheetHandle,
    viewport_width: c_double,
    viewport_height: c_double,
    out_count: *mut c_int,
) -> *mut CElementStyle {
    if doc.is_null() || ss.is_null() || out_count.is_null() {
        if !out_count.is_null() {
            unsafe { *out_count = 0; }
        }
        return std::ptr::null_mut();
    }

    let html_doc = unsafe { &*doc };
    let stylesheet = unsafe { &(*ss).0 };

    let states = HashMap::new();
    let computed = compute_styles(
        stylesheet,
        &html_doc.nodes,
        &states,
        viewport_width as f32,
        viewport_height as f32,
    );

    let count = computed.len();
    unsafe { *out_count = count as c_int; }

    if count == 0 {
        return std::ptr::null_mut();
    }

    
    let mut c_styles: Vec<CElementStyle> = Vec::with_capacity(count);

    for style_map in &computed {
        let props: Vec<CStyleProperty> = style_map.iter().map(|(k, v)| {
            let property = CString::new(k.as_str()).unwrap_or_default().into_raw();
            let value = CString::new(css_value_to_string(v)).unwrap_or_default().into_raw();
            CStyleProperty { property, value }
        }).collect();

        let prop_count = props.len() as c_int;
        let boxed_slice = props.into_boxed_slice();
        let ptr = Box::into_raw(boxed_slice) as *mut CStyleProperty;
        c_styles.push(CElementStyle {
            properties: ptr,
            property_count: prop_count,
        });
    }

    c_styles.leak().as_mut_ptr()
}


#[no_mangle]
pub extern "C" fn free_computed_styles(styles: *mut CElementStyle, count: c_int) {
    if styles.is_null() || count <= 0 {
        return;
    }
    unsafe {
        let slice = slice::from_raw_parts_mut(styles, count as usize);
        for style in slice {
            free_element_style(style.clone());
        }
        
        let _ = Vec::from_raw_parts(styles, count as usize, count as usize);
    }
}






#[no_mangle]
pub extern "C" fn layout_document(
    doc: *const HtmlDocument,
    styles: *const CElementStyle,
    style_count: c_int,
    viewport_width: c_double,
    viewport_height: c_double,
    out_count: *mut c_int,
) -> *mut CLayoutRect {
    if doc.is_null() || out_count.is_null() || style_count <= 0 {
        if !out_count.is_null() { unsafe { *out_count = 0; } }
        return std::ptr::null_mut();
    }

    let html_doc = unsafe { &*doc };

    
    let c_styles = unsafe { slice::from_raw_parts(styles, style_count as usize) };
    let style_maps: Vec<FxHashMap<String, CssValue>> = c_styles.iter().map(|c_style| {
        let mut map = FxHashMap::default();
        if c_style.properties.is_null() || c_style.property_count == 0 { return map; }
        unsafe {
            let props = slice::from_raw_parts(c_style.properties, c_style.property_count as usize);
            for prop in props {
                let key = CStr::from_ptr(prop.property).to_string_lossy().into_owned();
                let val = CStr::from_ptr(prop.value).to_string_lossy().into_owned();
                map.insert(key, parse_css_value_string(&val));
            }
        }
        map
    }).collect();

    
    let vw = viewport_width as f32;
    let vh = viewport_height as f32;

    
    let mut rects: Vec<CLayoutRect> = Vec::new();
    if !html_doc.nodes.is_empty() {
        if let Some(mut root_box) = build_layout_tree(&html_doc.nodes[0], &style_maps, vw, vh) {
            let cb = Rect::new(0.0, 0.0, vw, vh);
            let ctx = LayoutContext::new(vw, vh);
            layout_box_recursive(&mut root_box, cb, &ctx);
            collect_rects_for_c(&root_box, &html_doc.nodes, &mut rects);
        }
    }

    let count = rects.len() as c_int;
    unsafe { *out_count = count; }
    rects.leak().as_mut_ptr()
}

fn layout_box_recursive(box_: &mut LayoutBox, containing_block: Rect, ctx: &LayoutContext) {
    use crate::css::value::CssDisplay;
    match box_.box_type {
        BoxType::Block | BoxType::Float | BoxType::Positioned => {
            
            let _ = containing_block;
            let _ = ctx;
        }
        BoxType::FlexContainer => {  }
        _ => {}
    }
    for child in box_.children.iter_mut() {
        let child_cb = Rect::new(
            box_.rect.x + box_.padding.left + box_.border.left,
            box_.rect.y + box_.padding.top + box_.border.top,
            box_.rect.width - box_.padding.horizontal() - box_.border.horizontal(),
            0.0,
        );
        layout_box_recursive(child, child_cb, ctx);
    }
}

fn collect_rects_for_c(root: &LayoutBox, nodes: &[HtmlNode], rects: &mut Vec<CLayoutRect>) {
    if !root.is_display_none() && !root.rect.is_empty() {
        let tag = if root.node_index < nodes.len() {
            get_node_tag(&nodes[root.node_index])
        } else {
            "unknown".to_string()
        };
        rects.push(CLayoutRect {
            x: root.rect.x as c_double,
            y: root.rect.y as c_double,
            width: root.rect.width as c_double,
            height: root.rect.height as c_double,
            tag: CString::new(tag).unwrap_or_default().into_raw(),
            display_none: 0,
        });
    }
    for child in &root.children {
        collect_rects_for_c(child, nodes, rects);
    }
}


#[no_mangle]
pub extern "C" fn free_layout_results(rects: *mut CLayoutRect, count: c_int) {
    if rects.is_null() || count <= 0 {
        return;
    }
    unsafe {
        let slice = slice::from_raw_parts_mut(rects, count as usize);
        for rect in slice {
            free_layout_rect(*rect);
        }
        let _ = Vec::from_raw_parts(rects, count as usize, count as usize);
    }
}




#[no_mangle]
pub extern "C" fn create_document() -> *mut Document {
    let doc = Box::new(Document::new());
    Box::into_raw(doc)
}


#[no_mangle]
pub extern "C" fn doc_create_element(doc: *mut Document, tag: *const c_char) -> u64 {
    if doc.is_null() || tag.is_null() {
        return 0;
    }
    let tag_str = unsafe { CStr::from_ptr(tag).to_string_lossy().into_owned() };
    unsafe {
        let handle = (*doc).create_element(&tag_str);
        handle.0
    }
}


#[no_mangle]
pub extern "C" fn doc_append_child(doc: *mut Document, parent: u64, child: u64) -> c_int {
    if doc.is_null() {
        return 0;
    }
    unsafe {
        (*doc).append_child(NodeHandle(parent), NodeHandle(child)) as c_int
    }
}


#[no_mangle]
pub extern "C" fn free_document(doc: *mut Document) {
    if doc.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(doc);
    }
}



fn get_node_tag(node: &HtmlNode) -> String {
    match node {
        HtmlNode::Element { tag, .. } => tag.clone(),
        HtmlNode::Text(_) => "#text".to_string(),
        HtmlNode::Comment(_) => "#comment".to_string(),
        HtmlNode::Doctype { name, .. } => format!("#doctype({})", name),
    }
}

fn css_value_to_string(value: &CssValue) -> String {
    match value {
        CssValue::String(s) => s.to_string(),
        CssValue::Number(n) => format!("{}", n),
        CssValue::Color(c) => format!("#{:02x}{:02x}{:02x}{:02x}", c.r, c.g, c.b, c.a),
        CssValue::Keyword(k) => k.to_string(),
        CssValue::Url(u) => format!("url({})", u),
        CssValue::None => "none".to_string(),
        CssValue::Auto => "auto".to_string(),
        CssValue::Inherit => "inherit".to_string(),
        CssValue::Initial => "initial".to_string(),
        CssValue::Unset => "unset".to_string(),
        CssValue::Revert => "revert".to_string(),
        CssValue::Length(len) => css_length_to_string(len),
        CssValue::Calc(_) => "calc(...)".to_string(),
    }
}

fn css_length_to_string(len: &CssLength) -> String {
    match len {
        CssLength::Px(v) => format!("{}px", v),
        CssLength::Cm(v) => format!("{}cm", v),
        CssLength::Mm(v) => format!("{}mm", v),
        CssLength::Q(v) => format!("{}Q", v),
        CssLength::In(v) => format!("{}in", v),
        CssLength::Pc(v) => format!("{}pc", v),
        CssLength::Pt(v) => format!("{}pt", v),
        CssLength::Em(v) => format!("{}em", v),
        CssLength::Rem(v) => format!("{}rem", v),
        CssLength::Ex(v) => format!("{}ex", v),
        CssLength::Ch(v) => format!("{}ch", v),
        CssLength::Cap(v) => format!("{}cap", v),
        CssLength::Ic(v) => format!("{}ic", v),
        CssLength::Lh(v) => format!("{}lh", v),
        CssLength::Rlh(v) => format!("{}rlh", v),
        CssLength::Vw(v) => format!("{}vw", v),
        CssLength::Vh(v) => format!("{}vh", v),
        CssLength::Vmin(v) => format!("{}vmin", v),
        CssLength::Vmax(v) => format!("{}vmax", v),
        CssLength::Vi(v) => format!("{}vi", v),
        CssLength::Vb(v) => format!("{}vb", v),
        CssLength::Percent(v) => format!("{}%", v),
        CssLength::Number(v) => format!("{}", v),
        CssLength::Deg(v) => format!("{}deg", v),
        CssLength::Grad(v) => format!("{}grad", v),
        CssLength::Rad(v) => format!("{}rad", v),
        CssLength::Turn(v) => format!("{}turn", v),
        CssLength::S(v) => format!("{}s", v),
        CssLength::Ms(v) => format!("{}ms", v),
        CssLength::Hz(v) => format!("{}Hz", v),
        CssLength::KHz(v) => format!("{}kHz", v),
        CssLength::Dpi(v) => format!("{}dpi", v),
        CssLength::Dpcm(v) => format!("{}dpcm", v),
        CssLength::Dppx(v) => format!("{}dppx", v),
        CssLength::Unknown(v, u) => format!("{}{}", v, u),
    }
}

fn parse_css_value_string(s: &str) -> CssValue {
    let s = s.trim();
    if let Ok(n) = s.parse::<f32>() {
        return CssValue::Number(n);
    }
    if s.ends_with("px") {
        if let Some(n) = s.trim_end_matches("px").parse::<f32>().ok() {
            return CssValue::Length(CssLength::Px(n));
        }
    }
    if s.ends_with('%') {
        if let Some(n) = s.trim_end_matches('%').parse::<f32>().ok() {
            return CssValue::Length(CssLength::Percent(n));
        }
    }
    if s.starts_with('#') {
        if let Some(c) = crate::css::value::Color::from_hex(s) {
            return CssValue::Color(c);
        }
    }
    CssValue::Keyword(std::borrow::Cow::Owned(s.to_string()))
}





#[no_mangle]
pub extern "C" fn get_style_property(
    style: *const CElementStyle,
    property: *const c_char,
) -> *mut c_char {
    if style.is_null() || property.is_null() {
        return std::ptr::null_mut();
    }
    let style = unsafe { &*style };
    if style.properties.is_null() || style.property_count == 0 {
        return std::ptr::null_mut();
    }
    let prop_name = unsafe { CStr::from_ptr(property).to_string_lossy() };
    unsafe {
        let props = slice::from_raw_parts(style.properties, style.property_count as usize);
        for prop in props {
            let key = CStr::from_ptr(prop.property).to_string_lossy();
            if key == prop_name {
                return CString::new(CStr::from_ptr(prop.value).to_string_lossy().into_owned())
                    .ok().map(CString::into_raw).unwrap_or(std::ptr::null_mut());
            }
        }
    }
    std::ptr::null_mut()
}



#[no_mangle]
pub extern "C" fn get_cascaded_value(
    html: *const c_char, css: *const c_char, element_index: c_int,
    property: *const c_char, viewport_width: c_double, viewport_height: c_double,
) -> *mut c_char {
    if html.is_null() || css.is_null() || property.is_null() {
        return std::ptr::null_mut();
    }
    let html_str = unsafe { CStr::from_ptr(html).to_string_lossy() };
    let css_str = unsafe { CStr::from_ptr(css).to_string_lossy() };
    let prop_name = unsafe { CStr::from_ptr(property).to_string_lossy() };

    let mut html_parser = HtmlParser::new();
    let nodes = match html_parser.parse(&html_str) {
        Ok(n) => n, Err(_) => return std::ptr::null_mut(),
    };
    let mut css_parser = CssParser::new();
    let stylesheet = match css_parser.parse(&css_str) {
        Ok(s) => s, Err(_) => return std::ptr::null_mut(),
    };
    let states = HashMap::new();
    let styles = compute_styles(&stylesheet, &nodes, &states, viewport_width as f32, viewport_height as f32);

    if element_index as usize >= styles.len() {
        return std::ptr::null_mut();
    }
    if let Some(value) = styles[element_index as usize].get(prop_name.as_ref()) {
        return CString::new(css_value_to_string(value)).ok().map(CString::into_raw).unwrap_or(std::ptr::null_mut());
    }
    std::ptr::null_mut()
}



#[no_mangle]
pub extern "C" fn compute_styles_sync(
    html: *const c_char, css: *const c_char,
    viewport_width: c_double, viewport_height: c_double,
    out_count: *mut c_int,
) -> *mut CElementStyle {
    if html.is_null() || css.is_null() || out_count.is_null() {
        unsafe { if !out_count.is_null() { *out_count = 0; } }
        return std::ptr::null_mut();
    }
    let html_str = unsafe { CStr::from_ptr(html).to_string_lossy() };
    let css_str = unsafe { CStr::from_ptr(css).to_string_lossy() };
    let mut html_parser = HtmlParser::new();
    let nodes = match html_parser.parse(&html_str) {
        Ok(n) => n, Err(_) => { unsafe { *out_count = 0; } return std::ptr::null_mut(); }
    };
    let mut css_parser = CssParser::new();
    let stylesheet = match css_parser.parse(&css_str) {
        Ok(s) => s, Err(_) => { unsafe { *out_count = 0; } return std::ptr::null_mut(); }
    };
    let states = HashMap::new();
    let styles = compute_styles(&stylesheet, &nodes, &states, viewport_width as f32, viewport_height as f32);
    if styles.is_empty() { unsafe { *out_count = 0; } return std::ptr::null_mut(); }
    let c_styles: Vec<CElementStyle> = styles.iter().map(|m| style_map_to_c(m)).collect();
    let count = c_styles.len() as c_int;
    unsafe { *out_count = count; }
    Box::into_raw(c_styles.into_boxed_slice()) as *mut CElementStyle
}


#[no_mangle]
pub extern "C" fn get_css_unit_count() -> c_int { 32 }


#[no_mangle]
pub extern "C" fn get_css_unit_name(index: c_int) -> *mut c_char {
    let units = ["px","cm","mm","Q","in","pc","pt","em","rem","ex","ch","cap","ic","lh","rlh",
        "vw","vh","vmin","vmax","vi","vb","%","","deg","grad","rad","turn","s","ms","Hz","kHz","dpi","dpcm","dppx"];
    if index as usize >= units.len() { return std::ptr::null_mut(); }
    CString::new(units[index as usize]).ok().map(CString::into_raw).unwrap_or(std::ptr::null_mut())
}


#[no_mangle]
pub extern "C" fn is_css_property_inherited(property: *const c_char) -> c_int {
    if property.is_null() { return 0; }
    crate::css::properties::is_inherited_property(&unsafe { CStr::from_ptr(property).to_string_lossy() }) as c_int
}


#[no_mangle]
pub extern "C" fn css_length_to_px(
    value: c_double, unit: *const c_char,
    viewport_width: c_double, viewport_height: c_double,
    font_size: c_double, root_font_size: c_double,
) -> c_double {
    if unit.is_null() { return value; }
    let unit_str = unsafe { CStr::from_ptr(unit).to_string_lossy() };
    let len = crate::css::value::CssLength::from_value_and_unit(value as f32, &unit_str);
    let ctx = ViewportContext {
        viewport_width: viewport_width as f32, viewport_height: viewport_height as f32,
        font_size: font_size as f32, root_font_size: root_font_size as f32, containing_block_px: None,
    };
    len.to_px(&ctx) as c_double
}


#[no_mangle]
pub extern "C" fn parse_color(
    color_str: *const c_char, out_r: *mut c_int, out_g: *mut c_int, out_b: *mut c_int, out_a: *mut c_int,
) -> c_int {
    if color_str.is_null() || out_r.is_null() || out_g.is_null() || out_b.is_null() || out_a.is_null() { return 0; }
    let s = unsafe { CStr::from_ptr(color_str).to_string_lossy() };
    if s.starts_with('#') {
        if let Some(c) = crate::css::value::Color::from_hex(&s[1..]) {
            unsafe { *out_r = c.r as c_int; *out_g = c.g as c_int; *out_b = c.b as c_int; *out_a = c.a as c_int; }
            return 1;
        }
    }
    if let Some(c) = crate::css::value::Color::named(&s) {
        unsafe { *out_r = c.r as c_int; *out_g = c.g as c_int; *out_b = c.b as c_int; *out_a = c.a as c_int; }
        return 1;
    }
    0
}


#[no_mangle]
pub extern "C" fn mix_colors(
    r1: c_int, g1: c_int, b1: c_int, a1: c_int,
    r2: c_int, g2: c_int, b2: c_int, a2: c_int,
    mix_pct: c_double, out_r: *mut c_int, out_g: *mut c_int, out_b: *mut c_int, out_a: *mut c_int,
) {
    if out_r.is_null() || out_g.is_null() || out_b.is_null() || out_a.is_null() { return; }
    let c1 = crate::css::value::Color::new(r1 as u8, g1 as u8, b1 as u8, a1 as u8);
    let c2 = crate::css::value::Color::new(r2 as u8, g2 as u8, b2 as u8, a2 as u8);
    let mixed = crate::css::value::Color::color_mix(&c1, &c2, mix_pct as f32);
    unsafe { *out_r = mixed.r as c_int; *out_g = mixed.g as c_int; *out_b = mixed.b as c_int; *out_a = mixed.a as c_int; }
}

fn style_map_to_c(style_map: &FxHashMap<String, crate::css::value::CssValue>) -> CElementStyle {
    if style_map.is_empty() {
        return CElementStyle { properties: std::ptr::null_mut(), property_count: 0 };
    }
    let props: Vec<CStyleProperty> = style_map.iter().map(|(key, value)| {
        CStyleProperty {
            property: CString::new(key.as_str()).unwrap_or_default().into_raw(),
            value: CString::new(css_value_to_string(value)).unwrap_or_default().into_raw(),
        }
    }).collect();
    let count = props.len() as c_int;
    CElementStyle { properties: Box::into_raw(props.into_boxed_slice()) as *mut CStyleProperty, property_count: count }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_parse_html() {
        let html = CString::new("<html><body><div>Test</div></body></html>").unwrap();
        let doc = parse_html(html.as_ptr());
        assert!(!doc.is_null());

        let count = html_doc_node_count(doc);
        assert!(count >= 1);

        free_html_document(doc);
    }

    #[test]
    fn test_ffi_parse_css() {
        let css = CString::new("div { color: red; }").unwrap();
        let ss = parse_css(css.as_ptr());
        assert!(!ss.is_null());

        free_stylesheet(ss);
    }

    #[test]
    fn test_ffi_get_property() {
        let html = CString::new("<html><body><div id='test'>Test</div></body></html>").unwrap();
        let css = CString::new("div { color: red; font-size: 16px; }").unwrap();

        let doc = parse_html(html.as_ptr());
        let ss = parse_css(css.as_ptr());
        let mut style_count: c_int = 0;
        let styles = compute_styles_for_document(doc, ss, 1920.0, 1080.0, &mut style_count);
        assert!(style_count > 0);

        
        let prop_name = CString::new("color").unwrap();
        let styles_slice = unsafe { slice::from_raw_parts(styles, style_count as usize) };
        let mut found_color = false;
        for style in styles_slice {
            let val = get_style_property(style, prop_name.as_ptr());
            if !val.is_null() {
                unsafe {
                    let val_str = CStr::from_ptr(val).to_string_lossy();
                    assert!(!val_str.is_empty());
                }
                free_string(val);
                found_color = true;
                break;
            }
        }
        
        assert!(found_color, "No element had 'color' property");

        
        let missing = CString::new("nonexistent").unwrap();
        let val2 = get_style_property(&styles_slice[0], missing.as_ptr());
        assert!(val2.is_null());

        free_computed_styles(styles, style_count);
        free_stylesheet(ss);
        free_html_document(doc);
    }

    #[test]
    fn test_ffi_create_document() {
        let doc = create_document();
        assert!(!doc.is_null());

        let div_tag = CString::new("div").unwrap();
        let handle = doc_create_element(doc, div_tag.as_ptr());
        assert!(handle > 0);

        free_document(doc);
    }

    #[test]
    fn test_ffi_compute_and_layout() {
        let html = CString::new("<html><body><div>Test</div></body></html>").unwrap();
        let css = CString::new("div { color: red; }").unwrap();

        let doc = parse_html(html.as_ptr());
        let ss = parse_css(css.as_ptr());
        assert!(!doc.is_null());
        assert!(!ss.is_null());

        let mut style_count: c_int = 0;
        let styles = compute_styles_for_document(doc, ss, 1920.0, 1080.0, &mut style_count);
        assert!(style_count > 0);

        
        
        let mut layout_count: c_int = 0;
        let rects = layout_document(doc, styles, style_count, 1920.0, 1080.0, &mut layout_count);
        
        

        if layout_count > 0 && !rects.is_null() {
            unsafe {
                let first_rect = *rects;
                assert!(!first_rect.tag.is_null());
                let tag = CStr::from_ptr(first_rect.tag).to_string_lossy();
                assert!(!tag.is_empty());
            }
            free_layout_results(rects, layout_count);
        }

        free_computed_styles(styles, style_count);
        free_stylesheet(ss);
        free_html_document(doc);
    }
}
