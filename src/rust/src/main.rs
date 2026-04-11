

use std::env;
use std::sync::mpsc;

fn init_logging() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    )
    .format_timestamp_secs()
    .format_module_path(false)
    .format_target(false)
    .init();
    log::info!("🌐 Atrium Browser logging initialized");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    std::panic::set_hook(Box::new(|panic_info| {
        log::error!("🔴 Atrium Browser panicked!");
        if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            log::error!("Error: {}", s);
            eprintln!("Error: {}", s);
        }
    }));

    log::info!("🌐 Atrium Browser v{}", atrium_core::VERSION);
    println!("🌐 Atrium Browser v{}", atrium_core::VERSION);
    atrium_core::init();

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--help" | "-h" => { print_help(); pause(); return Ok(()); }
            "--version" | "-v" => {
                println!("\nAtrium Browser v{}", atrium_core::VERSION);
                pause(); return Ok(());
            }
            "--test" => { run_self_test()?; pause(); return Ok(()); }
            "--gui" => {
                #[cfg(feature = "gui")]
                return start_gui();
                #[cfg(not(feature = "gui"))]
                { eprintln!("GUI not enabled. Build with: --features gui"); pause(); return Ok(()); }
            }
            url => { println!("Navigating to: {}", url); return Ok(()); }
        }
    } else {
        println!("\n✅ Browser core initialized!");
        println!("   Run with --gui for graphical interface");
        println!("   Run with --test for self-test");
    }
    pause();
    Ok(())
}



#[cfg(feature = "gui")]
fn start_gui() -> Result<(), Box<dyn std::error::Error>> {
    use eframe::egui;
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Atrium Browser"),
        ..Default::default()
    };
    eframe::run_native(
        "Atrium Browser", native_options,
        Box::new(|_cc| Ok(Box::new(AtriumGuiApp::new()))),
    ).map_err(|e| format!("eframe error: {:?}", e))?;
    Ok(())
}

#[cfg(feature = "gui")]
struct AtriumGuiApp {
    url: String,
    html_input: String,
    status: String,
    show_source: bool,
    layout_result: Option<atrium_core::layout::LayoutResult>,
    html_nodes: Option<Vec<atrium_core::html::HtmlNode>>,
    computed_styles: Option<Vec<rustc_hash::FxHashMap<String, atrium_core::CssValue>>>,
    page_size: (f32, f32),
    loading: bool,
    
    fetch_rx: Option<mpsc::Receiver<Result<String, String>>>,
}

#[cfg(feature = "gui")]
use atrium_core::CssValue;
#[cfg(feature = "gui")]
use atrium_core::Color as CssColor;

#[cfg(feature = "gui")]
impl AtriumGuiApp {
    fn new() -> Self {
        Self {
            url: "https:
            html_input: r#"<html>
<body>
    <h1>Atrium Browser</h1>
    <p>Welcome to <strong>Atrium</strong> - a browser built from scratch with Rust!</p>
    <div>
        <h2>Features</h2>
        <ul>
            <li>Custom HTML/CSS parser</li>
            <li>Parallel layout with Rayon</li>
            <li>Flexbox support</li>
            <li>JavaScript interpreter</li>
        </ul>
    </div>
    <p>Click Render to see the page!</p>
</body>
</html>"#.to_string(),
            status: "Ready".to_string(),
            show_source: false,
            layout_result: None,
            html_nodes: None,
            computed_styles: None,
            page_size: (1200.0, 800.0),
            loading: false,
            fetch_rx: None,
        }
    }

    fn render_page(&mut self) {
        use atrium_core::HtmlParser;
        use atrium_core::css::parser::CssParser;
        use atrium_core::css::matcher::compute_styles;
        use atrium_core::layout::LayoutEngine;

        let mut hp = HtmlParser::new();
        match hp.parse(&self.html_input) {
            Ok(nodes) => {
                log::info!("✅ Parsed {} HTML nodes", nodes.len());

                
                let mut full_css = String::from(
                    "body { font-family: Arial; margin: 10px; color: #333; } \
                    h1 { font-size: 24px; color: #2c3e50; } \
                    h2 { font-size: 20px; color: #34495e; } \
                    p { font-size: 14px; } \
                    div { margin: 10px 0; } \
                    ul { margin: 5px 0; } \
                    li { margin: 3px 0; } \
                    strong { font-weight: bold; }"
                );
                for node in &nodes {
                    self.extract_css_from_nodes(node, &mut full_css);
                }
                log::debug!("📝 Full CSS length: {} chars", full_css.len());

                let mut cp = CssParser::new();
                match cp.parse(&full_css) {
                    Ok(ss) => {
                        log::info!("✅ Parsed {} CSS rules", ss.rules.len());
                        let states = std::collections::HashMap::new();
                        let styles = compute_styles(&ss, &nodes, &states, 1200.0, 2000.0);
                        log::info!("🎨 Computed {} style maps", styles.len());
                        let engine = LayoutEngine::new(1200.0, 2000.0);
                        let layout = engine.layout_with_stylesheet(&nodes, &ss);
                        log::info!("📐 Layout result: {}x{}", layout.rect.width, layout.rect.height);
                        self.layout_result = Some(layout);
                        self.html_nodes = Some(nodes);
                        self.computed_styles = Some(styles);
                        self.status = format!("✅ Rendered");
                    }
                    Err(e) => {
                        log::error!("❌ CSS parse error: {}", e);
                        self.status = format!("❌ CSS parse: {}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("❌ HTML parse error: {}", e);
                self.status = format!("❌ HTML parse: {}", e);
            }
        }
    }

    fn extract_css_from_nodes(&self, node: &atrium_core::html::HtmlNode, css: &mut String) {
        use atrium_core::html::HtmlNode;
        if let HtmlNode::Element { tag, children, .. } = node {
            if tag.to_lowercase() == "style" {
                for child in children {
                    if let HtmlNode::Text(text) = child {
                        css.push_str(text);
                        css.push('\n');
                    }
                }
            }
            for child in children {
                self.extract_css_from_nodes(child, css);
            }
        }
    }

    fn fetch_page(&mut self, ctx: egui::Context) {
        self.loading = true;
        self.status = format!("🔄 Loading {}...", self.url);
        let url = self.url.clone();
        let (tx, rx) = mpsc::channel();
        self.fetch_rx = Some(rx);

        std::thread::spawn(move || {
            match reqwest::blocking::get(&url) {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        match resp.text() {
                            Ok(body) => { let _ = tx.send(Ok(body)); }
                            Err(e) => { let _ = tx.send(Err(format!("Read error: {}", e))); }
                        }
                    } else {
                        let _ = tx.send(Err(format!("HTTP {}", status)));
                    }
                }
                Err(e) => { let _ = tx.send(Err(format!("Network: {}", e))); }
            }
            ctx.request_repaint();
        });
    }

    fn check_fetch_result(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.fetch_rx {
            if let Ok(result) = rx.try_recv() {
                self.loading = false;
                self.fetch_rx = None;
                match result {
                    Ok(html) => {
                        log::info!("✅ Fetched HTML, size: {} bytes", html.len());
                        self.html_input = html;
                        self.status = format!("✅ Loaded: {}", self.url);
                        self.render_page();
                        ctx.request_repaint();
                    }
                    Err(e) => {
                        self.status = format!("❌ {}", e);
                        self.html_input = format!(
                            "<html><body><h1>Error loading page</h1><p>{}</p></body></html>", e);
                        self.render_page();
                        ctx.request_repaint();
                    }
                }
            }
        }
    }
}

#[cfg(feature = "gui")]
impl eframe::App for AtriumGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.html_input.clear();
                        self.layout_result = None;
                        self.status = "New tab".into();
                        ui.close_menu();
                    }
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui.checkbox(&mut self.show_source, "Source").clicked() { ui.close_menu(); }
                });
            });
        });
        egui::TopBottomPanel::top("url_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("🌍");
                ui.text_edit_singleline(&mut self.url);
                let go_clicked = ui.button(if self.loading { "⏳" } else { "Go" }).clicked();
                if go_clicked && !self.loading {
                    self.fetch_page(ctx.clone());
                }
                if self.loading {
                    ui.spinner();
                    ui.label("Loading...");
                }
            });
        });

        
        self.check_fetch_result(ctx);
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.small(&self.status);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.show_source {
                ui.add_sized(ui.available_size(), egui::TextEdit::multiline(&mut self.html_input).code_editor());
            } else {
                ui.horizontal(|ui| {
                    if ui.button("▶ Render").clicked() { self.render_page(); ctx.request_repaint(); }
                });
                ui.separator();

                
                let nodes_opt = self.html_nodes.clone();
                let styles_opt = self.computed_styles.clone();

                if let (Some(nodes), Some(styles)) = (nodes_opt, styles_opt) {
                    let count = nodes.len();
                    let style_count = styles.len();

                    egui::ScrollArea::both().auto_shrink([false,false]).show(ui, |ui| {
                        ui.label(format!("✅ {} nodes, {} style maps", count, style_count));
                        ui.separator();
                        let mut idx = 0;
                        for node in &nodes {
                            render_node(ui, node, &styles, &mut idx);
                        }
                    });
                    self.status = format!("✅ {} nodes rendered", count);
                } else {
                    ui.centered_and_justified(|ui| ui.label("Click ▶ Render"));
                }
            }
        });
    }
}

#[cfg(feature = "gui")]
fn get_color_value(style: &rustc_hash::FxHashMap<String, CssValue>, prop: &str) -> Option<CssColor> {
    match style.get(prop) {
        Some(CssValue::Color(c)) => Some(*c),
        Some(CssValue::Keyword(k)) => CssColor::named(k.as_ref()),
        Some(CssValue::String(s)) if s.starts_with('#') => CssColor::from_hex(s.trim_start_matches('#')),
        Some(CssValue::String(s)) => CssColor::named(s.as_ref()),
        _ => None,
    }
}

#[cfg(feature = "gui")]
fn get_length_value(style: &rustc_hash::FxHashMap<String, CssValue>, prop: &str) -> Option<f32> {
    match style.get(prop) {
        Some(CssValue::Number(n)) => Some(*n),
        Some(CssValue::Length(l)) => Some(l.value()),
        _ => None,
    }
}


#[cfg(feature = "gui")]
fn extract_all_text(nodes: &[atrium_core::html::HtmlNode]) -> String {
    use atrium_core::html::HtmlNode;
    let mut text = String::new();
    for node in nodes {
        match node {
            HtmlNode::Text(t) => {
                let trimmed = t.trim();
                if !trimmed.is_empty() {
                    if !text.is_empty() { text.push(' '); }
                    text.push_str(trimmed);
                }
            }
            HtmlNode::Element { children, .. } => {
                let child_text = extract_all_text(children);
                if !child_text.is_empty() {
                    if !text.is_empty() { text.push(' '); }
                    text.push_str(&child_text);
                }
            }
            _ => {}
        }
    }
    text
}


#[cfg(feature = "gui")]
fn render_node(
    ui: &mut egui::Ui,
    node: &atrium_core::html::HtmlNode,
    styles: &[rustc_hash::FxHashMap<String, atrium_core::CssValue>],
    node_index: &mut usize,
) {
    match node {
        atrium_core::html::HtmlNode::Text(text) => {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                let ci = *node_index;
                let style = styles.get(ci);
                let color = style.and_then(|s| get_color_value(s, "color"))
                    .map(|c| egui::Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a))
                    .unwrap_or(egui::Color32::YELLOW);
                let font_size = style.and_then(|s| get_length_value(s, "font-size")).unwrap_or(14.0);
                let is_bold = style.and_then(|s| get_string_value(s, "font-weight"))
                    .map(|s| s == "bold" || s == "700").unwrap_or(false);
                let mut rich = egui::RichText::new(trimmed).color(color).size(font_size);
                if is_bold {
                    rich = rich.strong();
                }
                ui.label(rich);
            }
        }
        atrium_core::html::HtmlNode::Element { tag, children, .. } => {
            let tag_lower = tag.to_lowercase();
            let ci = *node_index;
            *node_index += 1;

            
            let style = styles.get(ci);
            let color = style.and_then(|s| get_color_value(s, "color"));
            let font_size = style.and_then(|s| get_length_value(s, "font-size"));
            let is_bold = style.and_then(|s| get_string_value(s, "font-weight"))
                .map(|s| s == "bold" || s == "700").unwrap_or(false);
            let bg = style.and_then(|s| get_color_value(s, "background-color"));

            
            if matches!(tag_lower.as_str(), "div" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "ul" | "ol" | "li") {
                ui.add_space(4.0);
            }

            if tag_lower == "br" {
                ui.add_space(12.0);
            } else if tag_lower == "hr" {
                ui.separator();
            } else if tag_lower == "h1" {
                let text = extract_all_text(children);
                if !text.is_empty() {
                    let mut rich = egui::RichText::new(&text).size(font_size.unwrap_or(28.0)).strong();
                    if let Some(c) = color {
                        rich = rich.color(egui::Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a));
                    }
                    ui.label(rich);
                } else {
                    for child in children { render_node(ui, child, styles, node_index); }
                }
            } else if tag_lower == "h2" {
                let text = extract_all_text(children);
                if !text.is_empty() {
                    let mut rich = egui::RichText::new(&text).size(font_size.unwrap_or(22.0)).strong();
                    if let Some(c) = color {
                        rich = rich.color(egui::Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a));
                    }
                    ui.label(rich);
                } else {
                    for child in children { render_node(ui, child, styles, node_index); }
                }
            } else if tag_lower == "h3" {
                let text = extract_all_text(children);
                if !text.is_empty() {
                    let mut rich = egui::RichText::new(&text).size(font_size.unwrap_or(18.0)).strong();
                    if let Some(c) = color {
                        rich = rich.color(egui::Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a));
                    }
                    ui.label(rich);
                } else {
                    for child in children { render_node(ui, child, styles, node_index); }
                }
            } else if tag_lower == "strong" || tag_lower == "b" {
                let text = extract_all_text(children);
                if !text.is_empty() {
                    let mut rich = egui::RichText::new(&text).strong();
                    if let Some(c) = color {
                        rich = rich.color(egui::Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a));
                    }
                    ui.label(rich);
                } else {
                    for child in children { render_node(ui, child, styles, node_index); }
                }
            } else if tag_lower == "a" {
                let text = extract_all_text(children);
                if !text.is_empty() {
                    let mut rich = egui::RichText::new(&text).color(egui::Color32::from_rgb(0, 0, 238));
                    if is_bold { rich = rich.strong(); }
                    ui.label(rich);
                } else {
                    for child in children { render_node(ui, child, styles, node_index); }
                }
            } else if tag_lower == "li" {
                ui.horizontal(|ui| {
                    ui.label("•");
                    for child in children { render_node(ui, child, styles, node_index); }
                });
            } else if tag_lower == "script" || tag_lower == "style" || tag_lower == "meta" || tag_lower == "link" || tag_lower == "head" {
                
            } else {
                
                for child in children {
                    render_node(ui, child, styles, node_index);
                }
            }

            
            if matches!(tag_lower.as_str(), "div" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6") {
                ui.add_space(8.0);
            }
        }
        _ => {}
    }
}

#[cfg(feature = "gui")]
fn get_string_value(style: &rustc_hash::FxHashMap<String, atrium_core::CssValue>, prop: &str) -> Option<String> {
    match style.get(prop) {
        Some(atrium_core::CssValue::Keyword(k)) => Some(k.to_string()),
        Some(atrium_core::CssValue::String(s)) => Some(s.to_string()),
        _ => None,
    }
}



fn print_help() {
    println!("\nAtrium Browser v{}\n\nUsage:\n  atrium [URL]\n  atrium --gui\n  atrium --test\n  atrium --help", atrium_core::VERSION);
}

fn run_self_test() -> Result<(), Box<dyn std::error::Error>> {
    use atrium_core::{Document, HtmlParser, YmdParser, SmlFile};
    println!("Testing DOM...");
    let mut doc = Document::new();
    let html = doc.create_element("html");
    let body = doc.create_element("body");
    doc.append_child(html, body);
    println!("  ✅ DOM OK");

    println!("Testing HTML Parser...");
    let mut p = HtmlParser::new();
    p.parse("<html><body></body></html>").map_err(|e| { eprintln!("❌ {}", e); e })?;
    println!("  ✅ HTML OK");

    println!("Testing YMD...");
    let yp = YmdParser::new().map_err(|e| { eprintln!("❌ {}", e); e })?;
    yp.parse("# T").map_err(|e| { eprintln!("❌ {}", e); e })?;
    println!("  ✅ YMD OK");

    println!("Testing SML...");
    let mut s = SmlFile::new();
    s.add_initial_key("t");
    s.validate().map_err(|e| { eprintln!("❌ {}", e); e })?;
    println!("  ✅ SML OK");

    println!("\n✅ All tests passed!");
    Ok(())
}

fn pause() {
    println!("\nPress Enter to exit...");
    let mut _input = String::new();
    let _ = std::io::stdin().read_line(&mut _input);
}
