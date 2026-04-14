//! Atrium Browser - Chrome-like GUI with full HTML/CSS/Layout rendering

use eframe::egui;
use egui_dock::{DockState, DockArea, TabViewer};
use crate::{HtmlParser, CssParser, CssValue, Color as CssColor, LayoutBox, BoxType, LayoutContext, build_layout_tree};
use crate::css::matcher::compute_styles;
use crate::layout::{layout_block_recursive, layout_flex_container, layout_inline_box, Rect as LayoutRect};
use crate::html::HtmlNode;
use std::collections::HashMap;
use std::sync::mpsc;
use log::info;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct TabId(u64);

#[derive(Clone, PartialEq)]
enum MyTab {
    Browser(TabId),
    Settings,
}

#[derive(Clone)]
struct RenderedElement {
    rect: LayoutRect,
    tag: String,
    text: Option<String>,
    bg_color: Option<egui::Color32>,
    text_color: Option<egui::Color32>,
    font_size: f32,
    is_bold: bool,
    is_italic: bool,
    url: Option<String>,
    image_src: Option<String>,
    border_color: Option<egui::Color32>,
    border_width: f32,
    padding: (f32, f32, f32, f32),
}

struct BrowserTab {
    id: TabId,
    url: String,
    url_input: String,
    title: String,
    html_content: String,
    status: String,
    loading: bool,
    rendered_elements: Vec<RenderedElement>,
    fetch_rx: Option<mpsc::Receiver<Result<String, String>>>,
    history: Vec<String>,
    history_index: isize,
}

impl BrowserTab {
    fn new(id: TabId, url: &str) -> Self {
        Self {
            id, url: url.to_string(), url_input: url.to_string(), title: String::new(),
            html_content: String::new(), status: String::from("Ready"), loading: false,
            rendered_elements: Vec::new(), fetch_rx: None,
            history: vec![url.to_string()], history_index: 0,
        }
    }
}

#[derive(Clone)]
struct AppSettings {
    dark_mode: bool, font_size: f32, zoom_level: f32, do_not_track: bool,
    block_third_party: bool, clear_on_exit: bool, hardware_acceleration: bool,
    max_tabs: usize, default_search: String, homepage: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self { dark_mode: false, font_size: 14.0, zoom_level: 1.0, do_not_track: false,
            block_third_party: true, clear_on_exit: false, hardware_acceleration: true,
            max_tabs: 50, default_search: "https://duckduckgo.com/?q=".to_string(),
            homepage: "about:atrium".to_string() }
    }
}

#[derive(Clone)]
enum ContextMenuAction { None, Back, Forward, Reload, ViewSource, Find, Settings }

struct AtriumApp {
    dock_state: DockState<MyTab>,
    browser_tabs: HashMap<TabId, BrowserTab>,
    next_tab_id: u64,
    settings: AppSettings,
    url_input: String,
    show_find: bool, find_input: String,
    context_menu_open: bool, context_menu_pos: egui::Pos2,
    context_menu_action: ContextMenuAction,
    maximized: bool, show_source: bool,
    image_cache: HashMap<String, Option<egui::TextureHandle>>,
    image_rx: Vec<mpsc::Receiver<(String, Result<Vec<u8>, String>)>>,
}

impl AtriumApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        Self::apply_chrome_theme(cc);
        let about_id = TabId(0);
        let mut browser_tabs = HashMap::new();
        let mut about_tab = BrowserTab::new(about_id, "about:atrium");
        about_tab.html_content = Self::get_about_html();
        about_tab.title = "About Atrium".to_string();
        browser_tabs.insert(about_id, about_tab);
        let dock_state = DockState::new(vec![MyTab::Browser(about_id)]);
        Self {
            dock_state, browser_tabs, next_tab_id: 1, settings: AppSettings::default(),
            url_input: String::new(), show_find: false, find_input: String::new(),
            context_menu_open: false, context_menu_pos: egui::Pos2::ZERO,
            context_menu_action: ContextMenuAction::None, maximized: false, show_source: false,
            image_cache: HashMap::new(), image_rx: Vec::new(),
        }
    }

    fn apply_chrome_theme(cc: &eframe::CreationContext) {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(20, 20, 25);
        visuals.window_fill = egui::Color32::from_rgb(25, 25, 32);
        visuals.extreme_bg_color = egui::Color32::from_rgb(15, 15, 20);
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(35, 35, 45);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(40, 40, 52);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(50, 50, 65);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(55, 55, 72);
        let accent = egui::Color32::from_rgb(66, 133, 244);
        visuals.selection.bg_fill = accent;
        visuals.selection.stroke = egui::Stroke::new(1.0, accent);
        visuals.hyperlink_color = accent;
        visuals.override_text_color = Some(egui::Color32::from_rgb(225, 228, 232));
        visuals.window_corner_radius = egui::CornerRadius::from(4);
        visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::from(4);
        visuals.widgets.inactive.corner_radius = egui::CornerRadius::from(4);
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.5, egui::Color32::from_rgb(50, 52, 58));
        cc.egui_ctx.set_visuals(visuals);
    }

    fn new_tab_id(&mut self) -> TabId {
        let id = TabId(self.next_tab_id); self.next_tab_id += 1; id
    }

    fn create_browser_tab(&mut self, url: &str) -> TabId {
        let id = self.new_tab_id();
        self.browser_tabs.insert(id, BrowserTab::new(id, url));
        let current_tabs: Vec<MyTab> = self.dock_state.main_surface().iter()
            .flat_map(|n| n.tabs().map(|t| t.to_vec()).unwrap_or_default()).collect();
        let mut all = current_tabs; all.push(MyTab::Browser(id));
        *self.dock_state.main_surface_mut() = egui_dock::Tree::new(all);
        id
    }

    fn resolve_url(base: &str, relative: &str) -> String {
        if relative.starts_with("http://") || relative.starts_with("https://") || relative.starts_with("file://") || relative.starts_with("data:") {
            return relative.to_string();
        }
        if relative.starts_with("//") { return format!("https:{}", relative); }
        if relative.starts_with('/') {
            if let Some(end) = base.find("://").and_then(|p| base[p+3..].find('/').map(|i| i+p+3)) {
                return format!("{}{}", &base[..end], relative);
            }
        }
        if let Some(last_slash) = base.rfind('/') {
            return format!("{}/{}", &base[..last_slash+1], relative);
        }
        format!("{}/{}", base, relative)
    }

    fn load_image(&mut self, ctx: &egui::Context, url: String) {
        if self.image_cache.contains_key(&url) { return; }
        self.image_cache.insert(url.clone(), None);
        let (tx, rx) = mpsc::channel();
        let url_clone = url.clone();
        std::thread::spawn(move || {
            let result: Result<Vec<u8>, String> = reqwest::blocking::get(&url_clone)
                .map_err(|e| e.to_string())
                .and_then(|r| r.bytes().map_err(|e| e.to_string()).map(|b| b.to_vec()));
            let _ = tx.send((url_clone, result));
        });
        self.image_rx.push(rx);
        ctx.request_repaint();
    }

    fn check_image_results(&mut self, ctx: &egui::Context) {
        let mut to_remove = Vec::new();
        for (i, rx) in self.image_rx.iter().enumerate() {
            if let Ok((url, result)) = rx.try_recv() {
                match result {
                    Ok(data) => {
                        if let Ok(image) = image::load_from_memory(&data) {
                            let rgba = image.to_rgba8();
                            let size = [rgba.width() as _, rgba.height() as _];
                            let texture = ctx.load_texture(
                                &url,
                                egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()),
                                egui::TextureOptions::LINEAR,
                            );
                            self.image_cache.insert(url, Some(texture));
                        }
                    }
                    Err(_) => { self.image_cache.insert(url, None); }
                }
                to_remove.push(i);
                ctx.request_repaint();
            }
        }
        for i in to_remove.into_iter().rev() {
            self.image_rx.remove(i);
        }
    }

    fn close_tab(&mut self, tab_id: &TabId) {
        self.browser_tabs.remove(tab_id);
        let remaining: Vec<MyTab> = self.dock_state.main_surface().iter()
            .flat_map(|n| n.tabs().map(|t| t.to_vec()).unwrap_or_default())
            .filter(|t| if let MyTab::Browser(tid) = t { tid != tab_id } else { true }).collect();
        let tabs = if remaining.is_empty() { vec![MyTab::Browser(self.new_tab_id())] } else { remaining };
        *self.dock_state.main_surface_mut() = egui_dock::Tree::new(tabs);
    }

    fn get_active_tab_id(&self) -> Option<TabId> {
        // Get the first node's first tab as active (simpler approach)
        for node in self.dock_state.main_surface().iter() {
            if let Some(tabs) = node.tabs() {
                if let Some(tab) = tabs.first() {
                    if let MyTab::Browser(id) = tab { return Some(*id); }
                }
            }
        }
        None
    }

    fn fetch_page(&mut self, ctx: egui::Context) {
        let tab_id = match self.get_active_tab_id() { Some(id) => id, None => return };
        let url = if let Some(tab) = self.browser_tabs.get(&tab_id) { tab.url_input.clone() } else { return };
        if let Some(tab) = self.browser_tabs.get_mut(&tab_id) {
            tab.loading = true; tab.status = format!("Loading {}...", url);
            let (tx, rx) = mpsc::channel(); tab.fetch_rx = Some(rx);
            std::thread::spawn(move || {
                match reqwest::blocking::get(&url) {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            match resp.text() {
                                Ok(body) => { let _ = tx.send(Ok(body)); }
                                Err(e) => { let _ = tx.send(Err(format!("Read error: {}", e))); }
                            }
                        } else { let _ = tx.send(Err(format!("HTTP {}", resp.status()))); }
                    }
                    Err(e) => { let _ = tx.send(Err(format!("Network: {}", e))); }
                }
                ctx.request_repaint();
            });
        }
    }

    fn check_fetch_result(&mut self, ctx: &egui::Context) {
        let tab_id = match self.get_active_tab_id() { Some(id) => id, None => return };
        let html_option: Option<Result<String, String>> = if let Some(tab) = self.browser_tabs.get(&tab_id) {
            tab.fetch_rx.as_ref().and_then(|rx| rx.try_recv().ok())
        } else { None };
        
        if let Some(result) = html_option {
            if let Some(tab) = self.browser_tabs.get_mut(&tab_id) {
                tab.loading = false; tab.fetch_rx = None;
                match &result {
                    Ok(html) => {
                        tab.html_content = html.clone(); tab.status = format!("Loaded: {}", tab.url);
                    }
                    Err(e) => {
                        tab.status = format!("Error: {}", e);
                        tab.html_content = format!("<html><body><h1>Error</h1><p>{}</p></body></html>", e);
                    }
                }
            }
            if let Ok(html) = result { self.render_html(&html, tab_id); }
            ctx.request_repaint();
        }
    }

    fn render_html(&mut self, html: &str, tab_id: TabId) {
        let mut hp = HtmlParser::new();
        let nodes = match hp.parse(html) { Ok(n) => n, Err(e) => { info!("HTML parse error: {}", e); return; } };
        let mut full_css = String::from(
            "body { font-family: Arial, sans-serif; margin: 8px; color: #e0e0e0; background-color: #1a1a22; line-height: 1.4; }
            h1 { font-size: 28px; margin: 16px 0 8px; color: #e0e0e0; } h2 { font-size: 22px; margin: 14px 0 6px; color: #d0d0d0; }
            h3 { font-size: 18px; margin: 12px 0 4px; color: #d0d0d0; } p { margin: 8px 0; color: #c8c8c8; }
            ul, ol { margin: 8px 0; padding-left: 24px; } li { margin: 2px 0; color: #c8c8c8; }
            a { color: #8ab4f8; text-decoration: underline; }
            strong, b { font-weight: bold; } em, i { font-style: italic; }
            code { font-family: monospace; background: #2d2d3a; padding: 1px 4px; border-radius: 3px; color: #e0e0e0; }
            pre { background: #252530; padding: 12px; border-radius: 4px; overflow-x: auto; color: #e0e0e0; }
            hr { border: none; border-top: 1px solid #3a3a4a; margin: 16px 0; } div { margin: 4px 0; }");
        for node in &nodes { self.extract_css_from_node(node, &mut full_css); }
        let mut cp = CssParser::new();
        let stylesheet = match cp.parse(&full_css) { Ok(ss) => ss, Err(_) => return };
        use crate::css::selector::ElementState;
        let states: HashMap<usize, ElementState> = HashMap::new();
        let computed = compute_styles(&stylesheet, &nodes, &states, 1200.0, 800.0);
        let mut elements = Vec::new();
        if let Some(root_node) = nodes.first() {
            if let Some(mut root_box) = build_layout_tree(root_node, &computed, 1200.0, 800.0) {
                let vp = LayoutRect::new(0.0, 0.0, 1200.0, 800.0);
                let ctx = LayoutContext::new(1200.0, 800.0);
                Self::layout_recursive_static(&mut root_box, vp, &ctx);
                Self::collect_elements_static(&root_box, &None, &mut elements);
            }
        }
        if let Some(tab) = self.browser_tabs.get_mut(&tab_id) {
            tab.rendered_elements = elements;
            tab.title = Self::extract_title(&nodes);
        }
    }

    fn extract_title(nodes: &[crate::html::HtmlNode]) -> String {
        for node in nodes {
            if let HtmlNode::Element { tag, children, .. } = node {
                if tag.to_lowercase() == "title" {
                    for child in children { if let HtmlNode::Text(text) = child { return text.trim().to_string(); } }
                }
                let child_title = Self::extract_title(children);
                if !child_title.is_empty() { return child_title; }
            }
        }
        String::new()
    }

    fn extract_css_from_node(&self, node: &HtmlNode, css: &mut String) {
        if let HtmlNode::Element { tag, children, .. } = node {
            if tag.to_lowercase() == "style" {
                for child in children { if let HtmlNode::Text(text) = child { css.push_str(text); css.push('\n'); } }
            }
            for child in children { self.extract_css_from_node(child, css); }
        }
    }

    fn layout_recursive_static(box_: &mut LayoutBox, cb: LayoutRect, ctx: &LayoutContext) {
        match &box_.box_type {
            BoxType::Block | BoxType::Positioned | BoxType::InlineBlock => { layout_block_recursive(box_, cb, ctx); }
            BoxType::FlexContainer => { layout_flex_container(box_, cb, ctx); }
            BoxType::Inline | BoxType::AnonymousText(_) => { layout_inline_box(box_, cb, ctx); }
            _ => { layout_block_recursive(box_, cb, ctx); }
        }
    }

    fn collect_elements_static(box_: &LayoutBox, current_href: &Option<String>, out: &mut Vec<RenderedElement>) {
        if !box_.rect.is_empty() {
            let style_ref = box_.style.as_ref();
            let bg_color = style_ref.and_then(|s| s.background_color.map(|c| egui::Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)));
            let text_color = style_ref.map(|s| egui::Color32::from_rgba_unmultiplied(s.color.r, s.color.g, s.color.b, s.color.a));
            let font_size = style_ref.map(|s| s.font_size).unwrap_or(14.0);
            let is_bold = style_ref.map(|s| s.font_weight >= 700.0).unwrap_or(false);
            let is_italic = style_ref.map(|s| matches!(s.font_style, crate::css::value::CssFontStyle::Italic)).unwrap_or(false);
            let border_width = style_ref.map(|s| s.border_left_width).unwrap_or(0.0);
            let border_color = style_ref.and_then(|s| if s.border_left_width > 0.0 { s.background_color.map(|c| egui::Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)) } else { None });
            let padding = style_ref.map(|s| (s.padding_top, s.padding_right, s.padding_bottom, s.padding_left)).unwrap_or((0.0, 0.0, 0.0, 0.0));
            let text = if let BoxType::AnonymousText(t) = &box_.box_type { Some(t.clone()) } else { None };
            let tag = match &box_.box_type { BoxType::Block => "div", BoxType::Inline => "span", BoxType::FlexContainer => "flex", _ => "element" }.to_string();
            let url = box_.url.clone().or_else(|| current_href.clone());
            let image_src = box_.image_src.clone();
            if bg_color.is_some() || text.is_some() || url.is_some() || image_src.is_some() || border_width > 0.0 {
                out.push(RenderedElement { rect: box_.rect, tag, text, bg_color, text_color, font_size, is_bold, is_italic, url, image_src, border_color, border_width, padding });
            }
        }
        let href_to_pass = if box_.url.is_some() { &box_.url } else { current_href };
        for child in &box_.children { Self::collect_elements_static(child, href_to_pass, out); }
    }

    fn get_about_html() -> String {
        r#"<html><head><style>
        body { max-width: 800px; margin: 40px auto; padding: 20px; font-family: 'Segoe UI', system-ui, sans-serif; color: #e0e0e0; background: #1a1a22; }
        h1 { font-size: 42px; color: #8ab4f8; margin-bottom: 8px; }
        h2 { font-size: 20px; color: #d0d0d0; margin-top: 24px; }
        .card { background: #252530; border: 1px solid #3a3a4a; border-radius: 8px; padding: 16px; margin: 12px 0; }
        .badge { display: inline-block; background: #2a2a40; color: #8ab4f8; padding: 4px 12px; border-radius: 16px; margin: 4px; font-size: 13px; }
        kbd { background: #2d2d3a; padding: 2px 6px; border-radius: 3px; font-family: monospace; font-size: 12px; color: #c8c8c8; }
        .shortcut { display: flex; justify-content: space-between; padding: 6px 0; border-bottom: 1px solid #3a3a4a; }
        </style></head><body>
        <h1>Atrium Browser</h1><p>Version 0.1.0 - High-performance multi-language browser</p>
        <div class="card"><h2>Features</h2><div>
        <span class="badge">Rust Core</span><span class="badge">HTML/CSS Engine</span>
        <span class="badge">Layout Engine</span><span class="badge">Tabbed Browsing</span>
        <span class="badge">Dark Theme</span></div></div>
        <div class="card"><h2>Keyboard Shortcuts</h2>
        <div class="shortcut"><span>New Tab</span><kbd>Ctrl+T</kbd></div>
        <div class="shortcut"><span>Close Tab</span><kbd>Ctrl+W</kbd></div>
        <div class="shortcut"><span>Find</span><kbd>Ctrl+F</kbd></div>
        <div class="shortcut"><span>Reload</span><kbd>F5</kbd></div></div>
        <div class="card"><h2>Tips</h2><p>Right-click for context menu. Enter any URL in the address bar.</p></div>
        </body></html>"#.to_string()
    }

    fn draw_title_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("titlebar").show(ctx, |ui| {
            let drag_resp = ui.interact(ui.max_rect(), ui.id().with("drag"), egui::Sense::drag());
            if drag_resp.dragged() { ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag); }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0; ui.add_space(8.0);
                ui.add(egui::Label::new(egui::RichText::new("").size(14.0)).selectable(false));
                ui.add_space(4.0);
                ui.add(egui::Label::new(egui::RichText::new("Atrium").size(12.0).color(egui::Color32::from_rgb(140, 145, 155))).selectable(false));
                let available = ui.available_width(); ui.add_space(available - 140.0);
                let btn_size = egui::vec2(46.0, 28.0);
                if self.window_btn(ui, "─", btn_size, egui::Color32::from_rgb(35, 35, 45)) { ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)); }
                let icon = if self.maximized { "❐" } else { "□" };
                if self.window_btn(ui, icon, btn_size, egui::Color32::from_rgb(35, 35, 45)) { self.maximized = !self.maximized; ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(self.maximized)); }
                if self.window_btn(ui, "✕", btn_size, egui::Color32::from_rgb(35, 35, 45)) { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
            });
        });
    }

    fn window_btn(&self, ui: &mut egui::Ui, text: &str, size: egui::Vec2, bg: egui::Color32) -> bool {
        let resp = ui.allocate_response(size, egui::Sense::click());
        let hovered = resp.hovered();
        let fill = if hovered { if text == "✕" { egui::Color32::from_rgb(232, 17, 35) } else { egui::Color32::from_rgb(55, 55, 72) } } else { bg };
        ui.painter().rect_filled(resp.rect, 0.0, fill);
        let color = if text == "✕" { egui::Color32::WHITE } else { egui::Color32::from_rgb(160, 165, 175) };
        ui.painter().text(resp.rect.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(11.0), color);
        resp.clicked()
    }

    fn draw_nav_bar(&mut self, ctx: &egui::Context) {
        let tab_id = self.get_active_tab_id();
        if let Some(id) = tab_id {
            if let Some(tab) = self.browser_tabs.get(&id) {
                if tab.url != "about:atrium" && tab.url != "about:blank" { self.url_input = tab.url_input.clone(); }
                else { self.url_input.clear(); }
            }
        }
        egui::TopBottomPanel::top("navbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let nav_btn = |ui: &mut egui::Ui, icon: &str, enabled: bool| {
                    let color = if enabled { egui::Color32::from_rgb(140, 145, 155) } else { egui::Color32::from_rgb(80, 80, 90) };
                    ui.add_enabled(enabled, egui::Button::new(egui::RichText::new(icon).size(14.0).color(color))
                        .fill(egui::Color32::TRANSPARENT).stroke(egui::Stroke::NONE).min_size(egui::vec2(28.0, 28.0)))
                };
                if nav_btn(ui, "←", self.can_go_back()).clicked() { self.go_back(); }
                if nav_btn(ui, "→", self.can_go_forward()).clicked() { self.go_forward(); }
                if nav_btn(ui, "↻", true).clicked() {
                    if let Some(id) = self.get_active_tab_id() {
                        if let Some(tab) = self.browser_tabs.get(&id) {
                            let url = tab.url.clone();
                            if !url.is_empty() && url != "about:atrium" && url != "about:blank" { self.navigate_to(&url); }
                        }
                    }
                }
                ui.add_space(4.0);
                let text_edit = egui::TextEdit::singleline(&mut self.url_input).hint_text("Search or enter web address").desired_width(f32::INFINITY);
                let response = ui.add(text_edit);
                if response.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let url = self.url_input.clone();
                    if !url.is_empty() { self.navigate_to(&url); }
                }
                ui.add_space(4.0);
                let url_to_navigate = self.url_input.clone();
                let accent = egui::Color32::from_rgb(66, 133, 244);
                if ui.add(egui::Button::new(egui::RichText::new("Go").size(13.0).color(egui::Color32::WHITE))
                    .fill(accent).corner_radius(egui::CornerRadius::from(4)).min_size(egui::vec2(56.0, 28.0))).clicked() {
                    if !url_to_navigate.is_empty() { self.navigate_to(&url_to_navigate); }
                }
            });
        });
    }

    fn navigate_to(&mut self, url: &str) {
        let url = url.trim().to_string(); if url.is_empty() { return; }
        let tab_id = match self.get_active_tab_id() { Some(id) => id, None => return };
        let base_url = if let Some(tab) = self.browser_tabs.get(&tab_id) { tab.url.clone() } else { String::new() };
        let resolved_url = if !url.starts_with("http://") && !url.starts_with("https://") && !url.starts_with("file://") && !url.starts_with("about:") && !url.starts_with("data:") {
            if !base_url.is_empty() { Self::resolve_url(&base_url, &url) }
            else { format!("https://{}", url) }
        } else { url };
        if let Some(tab) = self.browser_tabs.get_mut(&tab_id) {
            if (tab.history_index as usize) < tab.history.len() - 1 { tab.history.drain((tab.history_index as usize + 1)..); }
            tab.history.push(resolved_url.clone()); tab.history_index = (tab.history.len() - 1) as isize;
            tab.url = resolved_url.clone(); tab.url_input = resolved_url.clone();
        }
        if resolved_url.starts_with("about:") {
            if resolved_url == "about:atrium" {
                let html = Self::get_about_html();
                if let Some(tab) = self.browser_tabs.get_mut(&tab_id) { tab.html_content = html.clone(); tab.status = "Loaded: about:atrium".to_string(); }
                self.render_html(&html, tab_id);
            }
        } else {
            self.fetch_page(egui::Context::default());
        }
    }

    fn go_back(&mut self) {
        let tab_id = match self.get_active_tab_id() { Some(id) => id, None => return };
        if let Some(tab) = self.browser_tabs.get_mut(&tab_id) {
            if tab.history_index > 0 { tab.history_index -= 1; let url = tab.history[tab.history_index as usize].clone(); drop(tab); self.navigate_to(&url); }
        }
    }

    fn go_forward(&mut self) {
        let tab_id = match self.get_active_tab_id() { Some(id) => id, None => return };
        if let Some(tab) = self.browser_tabs.get_mut(&tab_id) {
            if (tab.history_index as usize) < tab.history.len() - 1 { tab.history_index += 1; let url = tab.history[tab.history_index as usize].clone(); drop(tab); self.navigate_to(&url); }
        }
    }

    fn can_go_back(&self) -> bool { if let Some(id) = self.get_active_tab_id() { if let Some(tab) = self.browser_tabs.get(&id) { return tab.history_index > 0; } } false }
    fn can_go_forward(&self) -> bool { if let Some(id) = self.get_active_tab_id() { if let Some(tab) = self.browser_tabs.get(&id) { return (tab.history_index as usize) < tab.history.len() - 1; } } false }

    fn draw_status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let status = if let Some(id) = self.get_active_tab_id() { self.browser_tabs.get(&id).map(|t| t.status.clone()).unwrap_or_default() } else { String::new() };
                ui.small(&status);
            });
        });
    }

    fn draw_find_bar(&mut self, ctx: &egui::Context) {
        if self.show_find {
            egui::TopBottomPanel::top("findbar").show(ctx, |ui| {
                ui.horizontal(|ui| { ui.label("🔍"); ui.text_edit_singleline(&mut self.find_input); if ui.button("Close").clicked() { self.show_find = false; } });
            });
        }
    }

    fn draw_context_menu(&mut self, ctx: &egui::Context) {
        if !self.context_menu_open { return; }
        let pos = self.context_menu_pos;
        let mut action = ContextMenuAction::None;
        egui::Area::new("ctx_menu".into()).fixed_pos(pos).order(egui::Order::Foreground).show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(200.0);
                if ui.button("← Back").clicked() { action = ContextMenuAction::Back; }
                if ui.button("→ Forward").clicked() { action = ContextMenuAction::Forward; }
                if ui.button("↻ Reload").clicked() { action = ContextMenuAction::Reload; }
                ui.separator();
                if ui.button("📋 View Source").clicked() { action = ContextMenuAction::ViewSource; }
                if ui.button("🔍 Find in Page").clicked() { action = ContextMenuAction::Find; }
                ui.separator();
                if ui.button("⚙ Settings").clicked() { action = ContextMenuAction::Settings; }
            });
        });
        if ctx.input(|i| i.pointer.any_click()) {
            let pointer_pos = ctx.input(|i| i.pointer.interact_pos());
            if let Some(ppos) = pointer_pos {
                let menu_rect = egui::Rect::from_min_max(pos, pos + egui::vec2(200.0, 250.0));
                if !menu_rect.contains(ppos) { self.context_menu_open = false; }
            }
        }
        match action {
            ContextMenuAction::Reload => { if let Some(id) = self.get_active_tab_id() { if let Some(tab) = self.browser_tabs.get(&id) { let url = tab.url.clone(); if !url.is_empty() && url != "about:atrium" { self.navigate_to(&url); } } } self.context_menu_open = false; }
            ContextMenuAction::ViewSource => { self.show_source = !self.show_source; self.context_menu_open = false; }
            ContextMenuAction::Find => { self.show_find = true; self.context_menu_open = false; }
            ContextMenuAction::Settings => {
                let current_tabs: Vec<MyTab> = self.dock_state.main_surface().iter().flat_map(|n| n.tabs().map(|t| t.to_vec()).unwrap_or_default()).collect();
                let mut all = current_tabs; all.push(MyTab::Settings);
                *self.dock_state.main_surface_mut() = egui_dock::Tree::new(all);
                self.context_menu_open = false;
            }
            _ => {}
        }
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::T) { self.create_browser_tab("about:blank"); }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::W) { if let Some(id) = self.get_active_tab_id() { self.close_tab(&id); } }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::F) { self.show_find = true; }
            if i.key_pressed(egui::Key::F5) { if let Some(id) = self.get_active_tab_id() { if let Some(tab) = self.browser_tabs.get(&id) { let url = tab.url.clone(); if !url.is_empty() && url != "about:atrium" { self.navigate_to(&url); } } } }
        });
    }

    fn draw_settings_panel(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::both().show(ui, |ui| { ui.heading("Settings"); ui.separator(); ui.collapsing("Appearance", |ui| { ui.checkbox(&mut self.settings.dark_mode, "Dark Mode"); ui.add(egui::Slider::new(&mut self.settings.font_size, 10.0..=24.0).text("Font Size")); }); ui.collapsing("Privacy", |ui| { ui.checkbox(&mut self.settings.do_not_track, "Do Not Track"); ui.checkbox(&mut self.settings.block_third_party, "Block Third-Party Cookies"); }); ui.separator(); if ui.button("Save").clicked() { info!("Settings saved"); } });
    }

    fn draw_about_panel(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::both().show(ui, |ui| { ui.heading("Atrium Browser"); ui.heading("Version 0.1.0"); ui.separator(); ui.label("High-performance multi-language browser"); ui.label("Rust Core, HTML/CSS/Layout Engines, Vulkan Renderer"); });
    }
}

impl eframe::App for AtriumApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sync_url_from_active_tab();
        self.handle_keyboard_shortcuts(ctx);
        self.check_fetch_result(ctx);
        self.check_image_results(ctx);
        self.draw_title_bar(ctx);
        self.draw_nav_bar(ctx);
        self.draw_find_bar(ctx);
        self.draw_status_bar(ctx);
        self.draw_context_menu(ctx);

        // DockArea с TabViewer для вкладок
        let mut style = egui_dock::Style::from_egui(ctx.style().as_ref());
        style.tab_bar.bg_fill = egui::Color32::from_rgb(20, 20, 25);
        style.tab.active.bg_fill = egui::Color32::from_rgb(25, 25, 32);
        style.tab.inactive.bg_fill = egui::Color32::from_rgb(35, 35, 45);
        
        // Split borrow: first render tabs, then let DockArea take over
        let active_tab_id = self.get_active_tab_id();
        let loading = active_tab_id.and_then(|id| self.browser_tabs.get(&id).map(|t| t.loading)).unwrap_or(false);
        
        egui::CentralPanel::default().show(ctx, |ui| {
            if loading {
                ui.centered_and_justified(|ui| { ui.spinner(); ui.label(" Loading..."); });
            } else if let Some(tab_id) = active_tab_id {
                if let Some(tab) = self.browser_tabs.get(&tab_id) {
                    let elements = tab.rendered_elements.clone();
                    let base_url = tab.url.clone();
                    let html = tab.html_content.clone();
                    let show_src = self.show_source;
                    
                    if show_src {
                        ui.add(egui::TextEdit::multiline(&mut html.clone()).code_editor().desired_width(f32::INFINITY));
                    } else if !elements.is_empty() {
                        egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                            for elem in &elements {
                                let rect = egui::Rect::from_min_size(egui::pos2(elem.rect.x, elem.rect.y), egui::vec2(elem.rect.width.max(1.0), elem.rect.height.max(1.0)));
                                if let Some(bg) = elem.bg_color { ui.painter().rect_filled(rect, 2.0, bg); }
                                if elem.border_width > 0.0 { let bc = elem.border_color.unwrap_or(egui::Color32::from_rgb(100, 100, 100)); ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(elem.border_width, bc), egui::StrokeKind::Inside); }
                                if let Some(ref img_src) = elem.image_src {
                                    let full_url = AtriumApp::resolve_url(&base_url, img_src);
                                    // Image loading handled in update via check_image_results
                                    if let Some(Some(texture)) = self.image_cache.get(&full_url) {
                                        ui.put(rect, egui::Image::new(texture));
                                    } else { ui.put(rect, egui::Label::new("🖼️")); }
                                    continue;
                                }
                                if let Some(ref text) = elem.text {
                                    if !text.trim().is_empty() {
                                        let mut rich_text = egui::RichText::new(text.trim()).size(elem.font_size);
                                        if let Some(tc) = elem.text_color { rich_text = rich_text.color(tc); }
                                        if elem.is_bold { rich_text = rich_text.strong(); }
                                        if elem.is_italic { rich_text = rich_text.italics(); }
                                        ui.put(rect, egui::Label::new(rich_text));
                                    }
                                }
                            }
                        });
                    } else if !html.is_empty() { ui.code(&html); }
                    else { ui.centered_and_justified(|ui| { ui.label("Empty page"); }); }
                }
            }
            
            // Tab bar at top
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 1.0;
                let tabs: Vec<MyTab> = self.dock_state.main_surface().iter()
                    .flat_map(|n| n.tabs().map(|t| t.to_vec()).unwrap_or_default()).collect();
                for tab in &tabs {
                    let label = match tab {
                        MyTab::Browser(tab_id) => {
                            if let Some(t) = self.browser_tabs.get(tab_id) {
                                if !t.title.is_empty() { let s = &t.title; if s.len() > 25 { &s[..25] } else { s.as_str() } }
                                else if t.url.is_empty() || t.url == "about:blank" { "New Tab" }
                                else { let s = t.url.trim_start_matches("https://").trim_start_matches("http://"); if s.len() > 25 { &s[..25] } else { s } }
                            } else { "Closed" }
                        }
                        MyTab::Settings => "Settings",
                    };
                    let is_active = true; // Simplified
                    let bg = if is_active { style.tab.active.bg_fill } else { style.tab.inactive.bg_fill };
                    ui.add(egui::Button::new(egui::RichText::new(label).size(12.0)).fill(bg).corner_radius(egui::CornerRadius::from(4)).min_size(egui::vec2(120.0, 28.0)));
                    if let MyTab::Browser(tid) = tab {
                        if ui.add(egui::Button::new("✕").fill(egui::Color32::TRANSPARENT).stroke(egui::Stroke::NONE).min_size(egui::vec2(16.0, 16.0))).clicked() {
                            self.browser_tabs.remove(tid);
                        }
                    }
                }
                if ui.add(egui::Button::new("+").fill(egui::Color32::TRANSPARENT).stroke(egui::Stroke::NONE).min_size(egui::vec2(24.0, 24.0))).clicked() {
                    let id = self.new_tab_id();
                    self.browser_tabs.insert(id, BrowserTab::new(id, "about:blank"));
                    let current_tabs: Vec<MyTab> = self.dock_state.main_surface().iter()
                        .flat_map(|n| n.tabs().map(|t| t.to_vec()).unwrap_or_default()).collect();
                    let mut all = current_tabs; all.push(MyTab::Browser(id));
                    *self.dock_state.main_surface_mut() = egui_dock::Tree::new(all);
                }
            });
        });
    }
}

impl AtriumApp {
    fn sync_url_from_active_tab(&mut self) {
        if let Some(id) = self.get_active_tab_id() {
            if let Some(tab) = self.browser_tabs.get(&id) {
                if tab.url != "about:atrium" && tab.url != "about:blank" {
                    self.url_input = tab.url_input.clone();
                } else {
                    self.url_input.clear();
                }
            }
        }
    }
}

impl TabViewer for AtriumApp {
    type Tab = MyTab;
    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            MyTab::Browser(tab_id) => {
                if let Some(t) = self.browser_tabs.get(tab_id) {
                    if !t.title.is_empty() { t.title.clone().into() }
                    else if !t.url.is_empty() && t.url != "about:blank" { t.url.clone().into() }
                    else { "New Tab".into() }
                } else { "Closed".into() }
            }
            MyTab::Settings => "Settings".into(),
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            MyTab::Browser(tab_id) => { self.draw_browser_content(ui, *tab_id); }
            MyTab::Settings => { self.draw_settings_panel(ui); }
        }
    }
    fn on_close(&mut self, tab: &mut Self::Tab) -> bool {
        if let MyTab::Browser(tab_id) = tab { self.close_tab(tab_id); true } else { false }
    }
}

impl AtriumApp {
    fn draw_browser_content(&mut self, ui: &mut egui::Ui, tab_id: TabId) {
        if let Some(tab) = self.browser_tabs.get(&tab_id) {
            if tab.loading { ui.centered_and_justified(|ui| { ui.spinner(); ui.label(" Loading..."); }); return; }
            if self.show_source { ui.add(egui::TextEdit::multiline(&mut tab.html_content.clone()).code_editor().desired_width(f32::INFINITY)); return; }
            if !tab.rendered_elements.is_empty() {
                let elements_to_check = tab.rendered_elements.clone();
                let base_url = tab.url.clone();
                let mut clicked_url: Option<String> = None;
                egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                    for elem in &elements_to_check {
                        let rect = egui::Rect::from_min_size(egui::pos2(elem.rect.x, elem.rect.y), egui::vec2(elem.rect.width.max(1.0), elem.rect.height.max(1.0)));
                        if let Some(bg) = elem.bg_color { ui.painter().rect_filled(rect, 2.0, bg); }
                        if elem.border_width > 0.0 { let bc = elem.border_color.unwrap_or(egui::Color32::from_rgb(100, 100, 100)); ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(elem.border_width, bc), egui::StrokeKind::Inside); }
                        if let Some(ref img_src) = elem.image_src {
                            let full_url = Self::resolve_url(&base_url, img_src);
                            self.load_image(ui.ctx(), full_url.clone());
                            if let Some(Some(texture)) = self.image_cache.get(&full_url) {
                                ui.put(rect, egui::Image::from_texture(texture));
                            } else {
                                ui.put(rect, egui::Label::new("🖼️"));
                            }
                            continue;
                        }
                        if let Some(ref text) = elem.text {
                            if !text.trim().is_empty() {
                                let mut rich_text = egui::RichText::new(text.trim()).size(elem.font_size);
                                if let Some(tc) = elem.text_color { rich_text = rich_text.color(tc); }
                                if elem.is_bold { rich_text = rich_text.strong(); }
                                if elem.is_italic { rich_text = rich_text.italics(); }
                                if let Some(ref url) = elem.url {
                                    let resolved = Self::resolve_url(&base_url, url);
                                    let link = egui::Hyperlink::from_label_and_url(rich_text, resolved.clone());
                                    let resp = ui.put(rect, link); if resp.clicked() { clicked_url = Some(resolved); }
                                } else { ui.put(rect, egui::Label::new(rich_text)); }
                            }
                        }
                    }
                });
                if let Some(url) = clicked_url { self.navigate_to(&url); }
            } else if !tab.html_content.is_empty() { ui.code(&tab.html_content); }
            else { ui.centered_and_justified(|ui| { ui.label("Empty page"); }); }
        }
    }
}

pub fn run_gui() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs().format_module_path(false).format_target(false).try_init().ok();
    info!("Starting Atrium Browser GUI...");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]).with_min_inner_size([800.0, 600.0]).with_decorations(false),
        ..Default::default()
    };
    eframe::run_native("Atrium Browser", options, Box::new(|cc| Ok(Box::new(AtriumApp::new(cc)))))
}
