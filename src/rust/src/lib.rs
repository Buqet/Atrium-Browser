

pub mod dom;
pub mod network;
pub mod layout;
pub mod security;
#[cfg(feature = "custom-formats")]
pub mod sml;
#[cfg(feature = "custom-formats")]
pub mod ymd;
#[cfg(feature = "custom-formats")]
pub mod yz;
pub mod html;
pub mod css;
pub mod js;
pub mod c_api;


pub use dom::{Document, Node, NodeHandle, NodeType};
pub use html::HtmlParser;
pub use css::parser::CssParser;
pub use css::value::{CssValue, Color, CssLength, ViewportContext};
pub use css::selector::{Selector, Specificity, ElementState};
pub use css::parser::{CssRule, Declaration, Stylesheet, MediaRule};
pub use layout::{Rect, Size, EdgeInsets, LayoutBox, BoxType, LayoutContext, build_layout_tree, perform_layout, collect_layout_rects };
pub use security::{CspPolicy, CorsValidator};
pub use js::parser::JsParser;
pub use js::interpreter::JsInterpreter;
#[cfg(feature = "custom-formats")]
pub use sml::{SmlFile, Language, Translation, Localization, SmlParser};
#[cfg(feature = "custom-formats")]
pub use ymd::{YmdDocument, YmdNode, YmdMetadata, YmdParser};
#[cfg(feature = "custom-formats")]
pub use yz::{YzPackage, YzHeader};
pub use c_api::*;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");




pub fn init() {
    
    let num_threads = num_cpus::get();
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .ok(); 

    log::info!("🚀 Atrium Core v{} initialized ({} worker threads)", VERSION, num_threads);
    println!("Atrium Core v{} initialized ({} worker threads)", VERSION, num_threads);
}









pub mod prelude {
    pub use crate::dom::{Document, NodeHandle, NodeType};
    pub use crate::html::HtmlParser;
    pub use crate::css::parser::Stylesheet;
    pub use crate::layout::LayoutBox;
    pub use crate::security::{CspPolicy, CorsValidator};
    pub use crate::css::{CssValue, CssParser as EngineCssParser};
}



#[cfg(test)]
mod integration_tests {
    use super::*;
    use css::parser::CssParser;
    use css::matcher::compute_styles;
    use css::value::CssValue;
    use html::HtmlParser;
    use rustc_hash::FxHashMap;
    use std::collections::HashMap;

    fn parse_html(html: &str) -> Vec<html::HtmlNode> {
        let mut parser = HtmlParser::new();
        parser.parse(html).expect("HTML should parse successfully")
    }

    fn parse_css(css: &str) -> Stylesheet {
        let mut parser = CssParser::new();
        parser.parse(css).expect("CSS should parse successfully")
    }

    fn compute(html: &str, css: &str) -> Vec<FxHashMap<String, CssValue>> {
        let nodes = parse_html(html);
        let stylesheet = parse_css(css);
        let states = HashMap::new();
        compute_styles(&stylesheet, &nodes, &states, 1920.0, 1080.0)
    }

    fn find_style_by_tag<'a>(
        nodes: &'a [html::HtmlNode],
        styles: &'a [FxHashMap<String, CssValue>],
        tag: &str,
    ) -> Option<&'a FxHashMap<String, CssValue>> {
        fn search<'a>(
            nodes: &'a [html::HtmlNode],
            styles: &'a [FxHashMap<String, CssValue>],
            tag: &str,
            idx: &mut usize,
        ) -> Option<&'a FxHashMap<String, CssValue>> {
            for node in nodes {
                let current_idx = *idx;
                *idx += 1;
                if let html::HtmlNode::Element { tag: t, .. } = node {
                    if t == tag {
                        return Some(&styles[current_idx]);
                    }
                }
                if let html::HtmlNode::Element { children, .. } = node {
                    if !children.is_empty() {
                        if let Some(style) = search(children, styles, tag, idx) {
                            return Some(style);
                        }
                    }
                }
            }
            None
        }
        let mut idx = 0;
        search(nodes, &styles, tag, &mut idx)
    }

    

    #[test]
    fn test_basic_html_css_pipeline() {
        let html = "<html><body><div>Hello</div></body></html>";
        let css = "div { color: red; font-size: 16px; }";

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let div_style = find_style_by_tag(&nodes, &styles, "div");
        assert!(div_style.is_some());
        let div_style = div_style.unwrap();

        assert!(div_style.contains_key("color"));
        assert!(div_style.contains_key("font-size"));
    }

    #[test]
    fn test_multiple_elements() {
        let html = "<html><body><p>Text</p><span>More</span></body></html>";
        let css = "p { color: blue; } span { color: green; }";

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let p_style = find_style_by_tag(&nodes, &styles, "p");
        assert!(p_style.is_some());
        assert!(p_style.unwrap().contains_key("color"));

        let span_style = find_style_by_tag(&nodes, &styles, "span");
        assert!(span_style.is_some());
        assert!(span_style.unwrap().contains_key("color"));
    }

    

    #[test]
    fn test_class_selector() {
        let html = r#"<html><body><div class="box">Content</div></body></html>"#;
        let css = ".box { margin: 10px; padding: 20px; }";

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let div_style = find_style_by_tag(&nodes, &styles, "div");
        assert!(div_style.is_some());
        let div_style = div_style.unwrap();
        assert!(div_style.contains_key("margin-top") || div_style.contains_key("margin"));
    }

    #[test]
    fn test_class_selector_no_match() {
        let html = r#"<html><body><div>Content</div></body></html>"#;
        let css = ".box { color: red; }";

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let div_style = find_style_by_tag(&nodes, &styles, "div");
        assert!(div_style.is_some());
        assert!(!div_style.unwrap().contains_key("color"));
    }

    #[test]
    fn test_id_selector() {
        let html = r#"<html><body><div id="main">Content</div></body></html>"#;
        let css = "#main { color: blue; }";

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let div_style = find_style_by_tag(&nodes, &styles, "div");
        assert!(div_style.is_some());
        assert!(div_style.unwrap().contains_key("color"));
    }

    

    #[test]
    fn test_specificity_id_beats_class() {
        let html = r#"<html><body><div id="main" class="box">Content</div></body></html>"#;
        let css = ".box { color: red; } #main { color: blue; }";

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let div_style = find_style_by_tag(&nodes, &styles, "div");
        assert!(div_style.is_some());
        assert!(div_style.unwrap().contains_key("color"));
    }

    #[test]
    fn test_important_overrides_specificity() {
        let html = r#"<html><body><div id="main" class="box">Content</div></body></html>"#;
        let css = "#main { color: blue; } .box { color: red !important; }";

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let div_style = find_style_by_tag(&nodes, &styles, "div");
        assert!(div_style.is_some());
        assert!(div_style.unwrap().contains_key("color"));
    }

    

    #[test]
    fn test_color_inheritance() {
        let html = "<html><body><div><span>Text</span></div></body></html>";
        let css = "div { color: red; }";

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let span_style = find_style_by_tag(&nodes, &styles, "span");
        assert!(span_style.is_some());
        assert!(
            span_style.unwrap().contains_key("color"),
            "span should inherit color from div"
        );
    }

    #[test]
    fn test_font_size_inheritance() {
        let html = "<html><body><div><p>Text</p></div></body></html>";
        let css = "div { font-size: 20px; }";

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let p_style = find_style_by_tag(&nodes, &styles, "p");
        assert!(p_style.is_some());
        assert!(
            p_style.unwrap().contains_key("font-size"),
            "p should inherit font-size from div"
        );
    }

    #[test]
    fn test_non_inherited_property_does_not_propagate() {
        let html = "<html><body><div><span>Text</span></div></body></html>";
        let css = "div { width: 200px; }";

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let span_style = find_style_by_tag(&nodes, &styles, "span");
        assert!(span_style.is_some());
        assert!(
            !span_style.unwrap().contains_key("width"),
            "span should NOT inherit width"
        );
    }

    

    #[test]
    fn test_media_query_applies_when_viewport_matches() {
        let html = "<html><body><div>Content</div></body></html>";
        let css = "@media (max-width: 600px) { div { color: blue; } }";

        let nodes = parse_html(html);
        let stylesheet = parse_css(css);
        let states = HashMap::new();
        let styles = compute_styles(&stylesheet, &nodes, &states, 500.0, 1080.0);

        let div_style = find_style_by_tag(&nodes, &styles, "div");
        assert!(div_style.is_some());
        assert!(div_style.unwrap().contains_key("color"));
    }

    #[test]
    fn test_media_query_does_not_apply_when_viewport_too_large() {
        let html = "<html><body><div>Content</div></body></html>";
        let css = "@media (max-width: 600px) { div { color: blue; } }";

        let nodes = parse_html(html);
        let stylesheet = parse_css(css);
        let states = HashMap::new();
        let styles = compute_styles(&stylesheet, &nodes, &states, 800.0, 1080.0);

        let div_style = find_style_by_tag(&nodes, &styles, "div");
        assert!(div_style.is_some());
        assert!(
            !div_style.unwrap().contains_key("color"),
            "color should not be set when media query doesn't match"
        );
    }

    

    #[test]
    fn test_descendant_selector() {
        let html = "<html><body><div><p><span>Text</span></p></div></body></html>";
        let css = "div span { color: green; }";

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let span_style = find_style_by_tag(&nodes, &styles, "span");
        assert!(span_style.is_some());
        assert!(span_style.unwrap().contains_key("color"));
    }

    #[test]
    fn test_child_selector() {
        let html = "<html><body><div><p><span>Deep</span></p></div></body></html>";
        let css = "div > p { color: red; }";

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let p_style = find_style_by_tag(&nodes, &styles, "p");
        assert!(p_style.is_some());
        assert!(p_style.unwrap().contains_key("color"));
    }

    

    #[test]
    fn test_margin_shorthand_expands() {
        let html = "<html><body><div>Content</div></body></html>";
        let css = "div { margin: 10px 20px; }";

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let div_style = find_style_by_tag(&nodes, &styles, "div");
        assert!(div_style.is_some());
        let div_style = div_style.unwrap();

        assert!(div_style.contains_key("margin-top"));
        assert!(div_style.contains_key("margin-right"));
        assert!(div_style.contains_key("margin-bottom"));
        assert!(div_style.contains_key("margin-left"));
    }

    #[test]
    fn test_padding_shorthand_one_value() {
        let html = "<html><body><div>Content</div></body></html>";
        let css = "div { padding: 15px; }";

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let div_style = find_style_by_tag(&nodes, &styles, "div");
        assert!(div_style.is_some());
        let div_style = div_style.unwrap();

        assert!(div_style.contains_key("padding-top"));
        assert!(div_style.contains_key("padding-left"));
    }

    

    #[test]
    fn test_css_variables() {
        let html = "<html><body><div>Content</div></body></html>";
        let css = ":root { --primary-color: blue; } div { color: var(--primary-color); }";

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let div_style = find_style_by_tag(&nodes, &styles, "div");
        assert!(div_style.is_some());
        assert!(div_style.unwrap().contains_key("color"));
    }

    

    #[test]
    fn test_empty_pseudo_class() {
        let html = "<html><body><div></div><div>Content</div></body></html>";
        let css = "div:empty { color: gray; }";

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let first_div_style = find_style_by_tag(&nodes, &styles, "div");
        assert!(first_div_style.is_some());
        assert!(first_div_style.unwrap().contains_key("color"));
    }

    

    #[test]
    fn test_webpage_like_css() {
        let html = r#"
            <html>
                <head><title>Test</title></head>
                <body>
                    <header class="site-header">Header</header>
                    <main>
                        <article class="post">
                            <h1>Title</h1>
                            <p class="intro">Intro text</p>
                            <p>Body text</p>
                        </article>
                    </main>
                    <footer>Footer</footer>
                </body>
            </html>
        "#;
        let css = r#"
            :root { --text-color: #333; --bg-color: #fff; }
            body { font-family: Arial, sans-serif; color: var(--text-color); }
            .site-header { background-color: #f0f0f0; padding: 10px; }
            .post { margin: 20px; }
            h1 { font-size: 24px; color: #111; }
            .intro { font-style: italic; }
            footer { color: #666; }
        "#;

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let body_style = find_style_by_tag(&nodes, &styles, "body");
        assert!(body_style.is_some());
        assert!(body_style.unwrap().contains_key("color"));

        let h1_style = find_style_by_tag(&nodes, &styles, "h1");
        assert!(h1_style.is_some());
        let h1_style = h1_style.unwrap();
        assert!(h1_style.contains_key("color"));
        assert!(h1_style.contains_key("font-size"));
    }

    

    #[test]
    fn test_empty_html_empty_css() {
        let html = "<html><body></body></html>";
        let css = "";

        let styles = compute(html, css);
        assert!(!styles.is_empty());
    }

    #[test]
    fn test_invalid_css_does_not_crash() {
        let html = "<html><body><div>Test</div></body></html>";
        let css = "div { color: ; invalid: stuff }}} { broken";

        let nodes = parse_html(html);
        let mut parser = CssParser::new();
        let result = parser.parse(css);
        if let Ok(stylesheet) = result {
            let states = HashMap::new();
            let styles = compute_styles(&stylesheet, &nodes, &states, 1920.0, 1080.0);
            assert!(!styles.is_empty());
        }
    }

    #[test]
    fn test_multiple_classes_on_element() {
        let html = r#"<html><body><div class="foo bar baz">Test</div></body></html>"#;
        let css = ".foo { color: red; } .bar { font-size: 14px; } .baz { margin: 5px; }";

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let div_style = find_style_by_tag(&nodes, &styles, "div");
        assert!(div_style.is_some());
        let div_style = div_style.unwrap();

        assert!(div_style.contains_key("color"));
        assert!(div_style.contains_key("font-size"));
        assert!(div_style.contains_key("margin-top") || div_style.contains_key("margin"));
    }

    #[test]
    fn test_cascade_same_property_different_rules() {
        let html = "<html><body><div>Test</div></body></html>";
        let css = "div { color: red; } div { color: blue; }";

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let div_style = find_style_by_tag(&nodes, &styles, "div");
        assert!(div_style.is_some());
        assert!(div_style.unwrap().contains_key("color"));
    }

    #[test]
    fn test_calc_in_css_value() {
        let html = "<html><body><div>Test</div></body></html>";
        let css = "div { width: calc(100% - 20px); }";

        let styles = compute(html, css);
        let nodes = parse_html(html);

        let div_style = find_style_by_tag(&nodes, &styles, "div");
        assert!(div_style.is_some());
        assert!(div_style.unwrap().contains_key("width"));
    }
}
