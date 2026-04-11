










use anyhow::{Result, anyhow, Context};
use pulldown_cmark::{Parser, Event, Tag, Options, CodeBlockKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use regex::Regex;


#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct YmdMetadata {
    
    pub title: Option<String>,
    
    pub author: Option<String>,
    
    pub date: Option<String>,
    
    pub tags: Vec<String>,
    
    pub extra: HashMap<String, String>,
}


#[derive(Debug, Clone)]
pub enum YmdNode {
    
    Paragraph(Vec<YmdInline>),
    
    Heading {
        level: u8,
        content: Vec<YmdInline>,
    },
    
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    
    BlockQuote(Vec<YmdNode>),
    
    List {
        ordered: bool,
        start: Option<u64>,
        items: Vec<Vec<YmdNode>>,
    },
    
    Note {
        content: String,
    },
    
    InternalLink {
        tag: String,
    },
    
    Image {
        alt: String,
        url: String,
        caption: Option<String>,
    },
    
    ThematicBreak,
    
    HtmlBlock(String),
    
    Text(String),
}


#[derive(Debug, Clone)]
pub enum YmdInline {
    Text(String),
    Bold(Vec<YmdInline>),
    Italic(Vec<YmdInline>),
    Code(String),
    Link { text: String, url: String },
    Note(String),
    InternalLink(String),
}


#[derive(Debug, Clone)]
pub struct YmdDocument {
    pub metadata: YmdMetadata,
    pub content: Vec<YmdNode>,
}

impl YmdDocument {
    
    pub fn new() -> Self {
        YmdDocument {
            metadata: YmdMetadata::default(),
            content: Vec::new(),
        }
    }

    
    pub fn with_title(title: &str) -> Self {
        let mut doc = YmdDocument::new();
        doc.metadata.title = Some(title.to_string());
        doc
    }
}

impl Default for YmdDocument {
    fn default() -> Self {
        Self::new()
    }
}


pub struct YmdParser {
    note_regex: Regex,
    link_regex: Regex,
    image_regex: Regex,
}

impl YmdParser {
    pub fn new() -> Result<Self> {
        Ok(YmdParser {
            note_regex: Regex::new(r"@note\(([^)]+)\)")?,
            link_regex: Regex::new(r"\[\[([^\]]+)\]\]")?,
            image_regex: Regex::new(r#"!\[([^\]]*)\]\(([^)]+)\)\s*\{caption="([^"]+)"\}"#)?,
        })
    }

    
    pub fn parse(&self, source: &str) -> Result<YmdDocument> {
        let mut doc = YmdDocument::new();
        
        
        let (frontmatter, content) = self.extract_frontmatter(source)?;
        
        
        doc.metadata = self.parse_metadata(&frontmatter)?;
        
        
        doc.content = self.parse_content(content)?;
        
        Ok(doc)
    }

    
    fn extract_frontmatter<'a>(&self, source: &'a str) -> Result<(Option<&'a str>, &'a str)> {
        if !source.starts_with("---") {
            return Ok((None, source));
        }

        
        if let Some(end) = source[3..].find("\n---") {
            let frontmatter = &source[4..end + 3];
            let content = &source[end + 7..];
            return Ok((Some(frontmatter), content));
        }

        Err(anyhow!("Invalid frontmatter: missing closing ---"))
    }

    
    fn parse_metadata(&self, frontmatter: &Option<&str>) -> Result<YmdMetadata> {
        let mut metadata = YmdMetadata::default();

        if let Some(fm) = frontmatter {
            
            if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(fm) {
                if let Some(map) = yaml.as_mapping() {
                    if let Some(title) = map.get("title").and_then(|v| v.as_str()) {
                        metadata.title = Some(title.to_string());
                    }
                    if let Some(author) = map.get("author").and_then(|v| v.as_str()) {
                        metadata.author = Some(author.to_string());
                    }
                    if let Some(date) = map.get("date").and_then(|v| v.as_str()) {
                        metadata.date = Some(date.to_string());
                    }
                    if let Some(tags) = map.get("tags").and_then(|v| v.as_sequence()) {
                        metadata.tags = tags
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();
                    }
                    
                    for (key, value) in map {
                        if let (Some(k), Some(v)) = (key.as_str(), value.as_str()) {
                            if !["title", "author", "date", "tags"].contains(&k) {
                                metadata.extra.insert(k.to_string(), v.to_string());
                            }
                        }
                    }
                }
            }
        }

        Ok(metadata)
    }

    
    fn parse_content(&self, content: &str) -> Result<Vec<YmdNode>> {
        let mut nodes = Vec::new();

        
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_TASKLISTS);

        let parser = Parser::new_ext(content, options);

        let mut current_inlines: Vec<YmdInline> = Vec::new();
        let mut list_items: Vec<Vec<YmdNode>> = Vec::new();
        let mut in_list = false;
        let mut list_ordered = false;
        let mut in_paragraph = false;
        let mut in_heading = false;
        let mut heading_level: u8 = 1;
        let mut in_blockquote = false;
        let mut in_code_block = false;
        let mut code_language: Option<String> = None;
        let mut code_content = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Paragraph) => {
                    in_paragraph = true;
                    current_inlines = Vec::new();
                }
                Event::End(Tag::Paragraph) => {
                    if in_paragraph && !current_inlines.is_empty() {
                        nodes.push(YmdNode::Paragraph(current_inlines.clone()));
                    }
                    in_paragraph = false;
                    current_inlines.clear();
                }
                Event::Start(Tag::Heading(level, _, _)) => {
                    in_heading = true;
                    heading_level = level as u8;
                    current_inlines = Vec::new();
                }
                Event::End(Tag::Heading(_, _, _)) => {
                    if in_heading {
                        nodes.push(YmdNode::Heading {
                            level: heading_level,
                            content: current_inlines.clone(),
                        });
                    }
                    in_heading = false;
                    current_inlines.clear();
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    in_code_block = true;
                    code_language = match kind {
                        CodeBlockKind::Fenced(lang) => {
                            if lang.is_empty() { None } else { Some(lang.to_string()) }
                        }
                        CodeBlockKind::Indented => None,
                    };
                    code_content = String::new();
                }
                Event::End(Tag::CodeBlock(_)) => {
                    if in_code_block {
                        nodes.push(YmdNode::CodeBlock { 
                            language: code_language.take(), 
                            code: code_content.clone() 
                        });
                    }
                    in_code_block = false;
                }
                Event::Start(Tag::BlockQuote) => {
                    in_blockquote = true;
                }
                Event::End(Tag::BlockQuote) => {
                    in_blockquote = false;
                }
                Event::Start(Tag::List(start)) => {
                    in_list = true;
                    list_ordered = start.is_some();
                    list_items = Vec::new();
                }
                Event::End(Tag::List(_)) => {
                    in_list = false;
                    nodes.push(YmdNode::List {
                        ordered: list_ordered,
                        start: None,
                        items: list_items.clone(),
                    });
                    list_items.clear();
                }
                Event::Start(Tag::Item) => {
                    if in_list {
                        current_inlines = Vec::new();
                    }
                }
                Event::End(Tag::Item) => {
                    if in_list && !current_inlines.is_empty() {
                        list_items.push(vec![YmdNode::Paragraph(current_inlines.clone())]);
                    }
                    current_inlines.clear();
                }
                Event::Start(Tag::Image(_link_type, alt_text, _title)) => {
                    let alt = alt_text.to_string();
                    nodes.push(YmdNode::Image {
                        alt,
                        url: String::new(), 
                        caption: None,
                    });
                }
                Event::Rule => {
                    nodes.push(YmdNode::ThematicBreak);
                }
                Event::Html(html) => {
                    nodes.push(YmdNode::HtmlBlock(html.to_string()));
                }
                Event::Text(text) => {
                    if in_code_block {
                        code_content.push_str(&text);
                    } else if in_paragraph || in_heading || in_blockquote {
                        
                        let inlines = self.parse_inlines(&text)?;
                        for inline in inlines {
                            current_inlines.push(YmdInline::Text(inline));
                        }
                    }
                }
                Event::Code(code) => {
                    if in_paragraph || in_heading {
                        current_inlines.push(YmdInline::Code(code.to_string()));
                    }
                }
                Event::SoftBreak | Event::HardBreak => {
                    if in_paragraph {
                        current_inlines.push(YmdInline::Text("\n".to_string()));
                    }
                }
                _ => {}
            }
        }

        Ok(nodes)
    }

    
    fn parse_inlines(&self, text: &str) -> Result<Vec<String>> {
        let mut result = Vec::new();
        let mut current = text.to_string();

        
        current = self.note_regex
            .replace_all(&current, "§NOTE:$1§")
            .to_string();

        
        current = self.link_regex
            .replace_all(&current, "§LINK:$1§")
            .to_string();

        
        for part in current.split('§') {
            if part.is_empty() {
                continue;
            }
            result.push(part.to_string());
        }

        Ok(result)
    }

    
    pub fn render_html(&self, doc: &YmdDocument) -> String {
        let mut html = String::new();

        
        if let Some(ref title) = doc.metadata.title {
            html.push_str(&format!("<h1>{}</h1>\n", escape_html(title)));
        }

        if !doc.metadata.tags.is_empty() {
            html.push_str("<div class=\"tags\">");
            for tag in &doc.metadata.tags {
                html.push_str(&format!(
                    "<a href=\"?tag={}\" class=\"tag\">{}</a>",
                    escape_html(tag),
                    escape_html(tag)
                ));
            }
            html.push_str("</div>\n");
        }

        
        for node in &doc.content {
            self.render_node_html(node, &mut html);
        }

        html
    }

    
    fn render_node_html(&self, node: &YmdNode, html: &mut String) {
        match node {
            YmdNode::Paragraph(inlines) => {
                html.push_str("<p>");
                for inline in inlines {
                    self.render_inline_html(inline, html);
                }
                html.push_str("</p>\n");
            }
            YmdNode::Heading { level, content } => {
                html.push_str(&format!("<h{}>", level));
                for inline in content {
                    self.render_inline_html(inline, html);
                }
                html.push_str(&format!("</h{}>\n", level));
            }
            YmdNode::CodeBlock { language, code } => {
                if let Some(lang) = language {
                    html.push_str(&format!(
                        "<pre><code class=\"language-{}\">{}</code></pre>\n",
                        escape_html(lang),
                        escape_html(code)
                    ));
                } else {
                    html.push_str(&format!(
                        "<pre><code>{}</code></pre>\n",
                        escape_html(code)
                    ));
                }
            }
            YmdNode::BlockQuote(children) => {
                html.push_str("<blockquote>");
                for child in children {
                    self.render_node_html(child, html);
                }
                html.push_str("</blockquote>\n");
            }
            YmdNode::List { ordered, items, .. } => {
                let tag = if *ordered { "ol" } else { "ul" };
                html.push_str(&format!("<{}>", tag));
                for item in items {
                    html.push_str("<li>");
                    for node in item {
                        self.render_node_html(node, html);
                    }
                    html.push_str("</li>");
                }
                html.push_str(&format!("</{}>\n", tag));
            }
            YmdNode::Note { content } => {
                html.push_str(&format!(
                    "<span class=\"note\" title=\"{}\">📝</span>\n",
                    escape_html(content)
                ));
            }
            YmdNode::InternalLink { tag } => {
                html.push_str(&format!(
                    "<a href=\"?tag={}\" class=\"internal-link\">{}</a>\n",
                    escape_html(tag),
                    escape_html(tag)
                ));
            }
            YmdNode::Image { alt, url, caption } => {
                html.push_str(&format!("<figure><img src=\"{}\" alt=\"{}\">", 
                    escape_html(url), escape_html(alt)));
                if let Some(cap) = caption {
                    html.push_str(&format!("<figcaption>{}</figcaption>", escape_html(cap)));
                }
                html.push_str("</figure>\n");
            }
            YmdNode::ThematicBreak => {
                html.push_str("<hr>\n");
            }
            YmdNode::HtmlBlock(html_content) => {
                html.push_str(html_content);
                html.push('\n');
            }
            YmdNode::Text(text) => {
                
                let processed = self.process_inline_text(text);
                html.push_str(&processed);
            }
        }
    }

    
    fn render_inline_html(&self, inline: &YmdInline, html: &mut String) {
        match inline {
            YmdInline::Text(text) => {
                html.push_str(&self.process_inline_text(text));
            }
            YmdInline::Bold(children) => {
                html.push_str("<strong>");
                for child in children {
                    self.render_inline_html(child, html);
                }
                html.push_str("</strong>");
            }
            YmdInline::Italic(children) => {
                html.push_str("<em>");
                for child in children {
                    self.render_inline_html(child, html);
                }
                html.push_str("</em>");
            }
            YmdInline::Code(code) => {
                html.push_str(&format!("<code>{}</code>", escape_html(code)));
            }
            YmdInline::Link { text, url } => {
                html.push_str(&format!(
                    "<a href=\"{}\">{}</a>",
                    escape_html(url),
                    escape_html(text)
                ));
            }
            YmdInline::Note(content) => {
                html.push_str(&format!(
                    "<span class=\"note\" title=\"{}\">📝</span>",
                    escape_html(content)
                ));
            }
            YmdInline::InternalLink(tag) => {
                html.push_str(&format!(
                    "<a href=\"?tag={}\" class=\"internal-link\">{}</a>",
                    escape_html(tag),
                    escape_html(tag)
                ));
            }
        }
    }

    
    fn process_inline_text(&self, text: &str) -> String {
        let mut result = text.to_string();

        
        result = self.note_regex
            .replace_all(&result, "<span class=\"note\" title=\"$1\">📝</span>")
            .to_string();

        
        result = self.link_regex
            .replace_all(&result, "<a href=\"?tag=$1\" class=\"internal-link\">$1</a>")
            .to_string();

        result
    }
}

impl Default for YmdParser {
    fn default() -> Self {
        Self::new().unwrap()
    }
}


pub fn parse_ymd(source: &str) -> Result<YmdDocument> {
    let parser = YmdParser::new()?;
    parser.parse(source)
}


pub fn render_ymd(doc: &YmdDocument) -> String {
    let parser = YmdParser::new().unwrap();
    parser.render_html(doc)
}


pub fn extract_metadata(source: &str) -> Result<YmdMetadata> {
    let parser = YmdParser::new()?;
    let doc = parser.parse(source)?;
    Ok(doc.metadata)
}


fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_document() {
        let source = r#"---
title: Test Note
author: John Doe
tags: [test, example]
---

# Hello World

This is a **test** document.

@note(Important information)

Check out [[rust]] for more info.
"#;

        let parser = YmdParser::new().unwrap();
        let doc = parser.parse(source).unwrap();

        assert_eq!(doc.metadata.title, Some("Test Note".to_string()));
        assert_eq!(doc.metadata.author, Some("John Doe".to_string()));
        assert_eq!(doc.metadata.tags, vec!["test", "example"]);
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let source = "# Just content\n\nNo frontmatter here.";
        
        let parser = YmdParser::new().unwrap();
        let doc = parser.parse(source).unwrap();

        assert!(doc.metadata.title.is_none());
    }

    #[test]
    fn test_render_html() {
        let mut doc = YmdDocument::new();
        doc.metadata.title = Some("Test".to_string());
        doc.content.push(YmdNode::Heading {
            level: 1,
            content: vec![YmdInline::Text("Hello".to_string())],
        });

        let parser = YmdParser::new().unwrap();
        let html = parser.render_html(&doc);

        assert!(html.contains("<h1>Test</h1>"));
        assert!(html.contains("<h1>Hello</h1>"));
    }

    #[test]
    fn test_internal_links() {
        let source = "# Links\n\nCheck out [[rust]] and [[web-development]].";

        let parser = YmdParser::new().unwrap();
        let doc = parser.parse(source).unwrap();

        
        assert!(!doc.content.is_empty());
    }

    #[test]
    fn test_notes() {
        let source = "# Note\n\nImportant @note(this is a note) here.";

        let parser = YmdParser::new().unwrap();
        let doc = parser.parse(source).unwrap();

        
        assert!(!doc.content.is_empty());
    }

    #[test]
    fn test_code_block() {
        let source = r#"
```rust
fn main() {
    println!("Hello!");
}
```
"#;

        let parser = YmdParser::new().unwrap();
        let doc = parser.parse(source).unwrap();

        assert!(doc.content.iter().any(|n| matches!(n, YmdNode::CodeBlock { language: Some(lang), .. } if lang == "rust")));
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("<script>"), "&lt;script&gt;");
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html("\"quotes\""), "&quot;quotes&quot;");
    }
}
