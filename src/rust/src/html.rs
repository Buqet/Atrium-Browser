use std::collections::HashMap;
use std::io::{self, Write};
use log::{info, debug, warn};


const MAX_DEPTH: usize = 100;


const MAX_NODES: usize = 100_000;


#[derive(Debug, Clone)]
pub enum HtmlNode {
    Element {
        tag: String,
        attributes: HashMap<String, String>,
        children: Vec<HtmlNode>,
    },
    Text(String),
    Comment(String),
    Doctype {
        name: String,
        public_id: Option<String>,
        system_id: Option<String>,
    },
}


pub struct HtmlParser {
    pub unsupported_patterns: Vec<String>,
    pub supported_tags: Vec<&'static str>,
    pub supported_attributes: Vec<&'static str>,
    node_count: usize,
}

impl HtmlParser {
    pub fn new() -> Self {
        debug!("HtmlParser initializing with {} supported tags",
            vec![
                "html", "head", "body", "div", "span", "p", "a", "img", "br", "hr",
                "h1", "h2", "h3", "h4", "h5", "h6", "ul", "ol", "li", "table", "tr",
                "td", "th", "thead", "tbody", "form", "input", "button", "textarea",
                "select", "option", "label", "script", "style", "link", "meta",
                "title", "header", "footer", "nav", "main", "section", "article",
                "aside", "details", "summary", "time", "code", "pre", "blockquote",
                "iframe", "svg", "path", "circle", "rect", "g", "use", "symbol",
                "canvas", "video", "audio", "source", "track", "picture", "figure",
                "figcaption", "mark", "small", "strong", "em", "u", "s", "del", "ins",
                "sub", "sup", "i", "b", "abbr", "cite", "dfn", "kbd", "samp", "var",
                "datalist", "fieldset", "legend", "output", "progress", "meter",
                "template", "slot", "wbr", "ruby", "rt", "rp", "bdi", "bdo",
            ].len()
        );
        HtmlParser {
            unsupported_patterns: Vec::new(),
            supported_tags: vec![
                "html", "head", "body", "div", "span", "p", "a", "img", "br", "hr",
                "h1", "h2", "h3", "h4", "h5", "h6", "ul", "ol", "li", "table", "tr",
                "td", "th", "thead", "tbody", "form", "input", "button", "textarea",
                "select", "option", "label", "script", "style", "link", "meta",
                "title", "header", "footer", "nav", "main", "section", "article",
                "aside", "details", "summary", "time", "code", "pre", "blockquote",
                "iframe", "svg", "path", "circle", "rect", "g", "use", "symbol",
                "canvas", "video", "audio", "source", "track", "picture", "figure",
                "figcaption", "mark", "small", "strong", "em", "u", "s", "del", "ins",
                "sub", "sup", "i", "b", "abbr", "cite", "dfn", "kbd", "samp", "var",
                "datalist", "fieldset", "legend", "output", "progress", "meter",
                "template", "slot", "wbr", "ruby", "rt", "rp", "bdi", "bdo",
            ],
            supported_attributes: vec![
                "id", "class", "style", "title", "lang", "dir", "hidden", "tabindex",
                "accesskey", "draggable", "spellcheck", "contenteditable", "data-*",
                "href", "src", "alt", "width", "height", "loading", "decoding",
                "type", "name", "value", "placeholder", "required", "disabled",
                "readonly", "checked", "selected", "multiple", "min", "max", "step",
                "pattern", "maxlength", "minlength", "autocomplete", "autofocus",
                "for", "action", "method", "enctype", "target", "rel", "download",
                "role", "aria-*", "aria-label", "aria-labelledby", "aria-describedby",
                "aria-hidden", "aria-expanded", "aria-controls", "aria-owns",
                "colspan", "rowspan", "headers", "scope", "caption", "col", "colgroup",
                "charset", "content", "http-equiv", "viewport", "description",
                "author", "keywords", "og:*", "twitter:*", "property", "itemprop",
                "slot", "part", "exportparts", "is", "itemid", "itemprop", "itemref",
                "itemscope", "itemtype", "translate", "inputmode", "nonce",
            ],
            node_count: 0,
        }
    }

    
    pub fn parse(&mut self, html: &str) -> Result<Vec<HtmlNode>, String> {
        self.unsupported_patterns.clear();
        self.node_count = 0;
        
        
        if html.trim().is_empty() {
            warn!("HtmlParser: empty HTML input");
            return Ok(Vec::new());
        }
        
        
        if html.len() > 10 * 1024 * 1024 {
            warn!("HtmlParser: HTML too large ({} bytes)", html.len());
            return Err(format!("HTML too large: {} bytes (max 10MB)", html.len()));
        }
        
        let result = self.parse_html(html);
        result
    }

    fn parse_html(&mut self, html: &str) -> Result<Vec<HtmlNode>, String> {
        let mut nodes = Vec::new();
        let chars: Vec<char> = html.chars().collect();
        let mut pos = 0;

        while pos < chars.len() {
            
            if self.node_count >= MAX_NODES {
                warn!("HtmlParser: exceeded max nodes limit ({})", MAX_NODES);
                break;
            }

            
            while pos < chars.len() && chars[pos].is_whitespace() {
                pos += 1;
            }

            if pos >= chars.len() {
                break;
            }

            let last_pos = pos;
            match self.parse_node(&chars, pos, "root", 0)? {
                Some((node, consumed)) => {
                    if consumed == 0 {
                        warn!("HtmlParser: zero consumed, breaking at pos {}", pos);
                        break;
                    }
                    nodes.push(node);
                    pos += consumed;
                    
                    if pos <= last_pos {
                        warn!("HtmlParser: position not advancing, breaking");
                        break;
                    }
                }
                None => {
                    
                    pos += 1;
                }
            }
        }

        info!("✅ HtmlParser::parse - {} nodes parsed", nodes.len());

        Ok(nodes)
    }

    fn parse_node(&mut self, chars: &[char], start: usize, parent_tag: &str, depth: usize) -> Result<Option<(HtmlNode, usize)>, String> {
        
        if depth > MAX_DEPTH {
            warn!("HtmlParser: exceeded max depth limit ({})", MAX_DEPTH);
            return Err(format!("Max nesting depth exceeded ({})", MAX_DEPTH));
        }

        let mut pos = start;

        
        while pos < chars.len() && chars[pos].is_whitespace() {
            pos += 1;
        }

        if pos >= chars.len() {
            return Ok(None);
        }

        match chars[pos] {
            '<' => {
                
                if pos + 1 < chars.len() && chars[pos + 1] == '/' {
                    return Ok(None);
                }

                pos += 1;
                if pos < chars.len() {
                    let result = match chars[pos] {
                        '!' => {
                            pos += 1;
                            if pos < chars.len() && chars[pos] == '-' {
                                self.parse_comment(chars, pos)
                            } else {
                                self.parse_doctype(chars, pos)
                            }
                        }
                        _ => self.parse_element(chars, pos, parent_tag, depth),
                    }?;

                    
                    if let Some((node, consumed_from_pos)) = result {
                        let total_consumed = (pos - start) + consumed_from_pos;
                        return Ok(Some((node, total_consumed)));
                    }
                    return Ok(None);
                } else {
                    Ok(None)
                }
            }
            _ => self.parse_text(chars, pos),
        }
    }

    fn parse_comment(&mut self, chars: &[char], start: usize) -> Result<Option<(HtmlNode, usize)>, String> {
        let mut pos = start;

        
        if pos < chars.len() && chars[pos] == '-' {
            pos += 1;
        }

        let mut comment = String::new();

        while pos < chars.len() {
            if chars[pos] == '-' && pos + 1 < chars.len() && chars[pos + 1] == '-' {
                
                
                if pos + 2 < chars.len() && chars[pos + 2] == '>' {
                    pos += 3;
                    let consumed = pos - start;
                    return Ok(Some((HtmlNode::Comment(comment), consumed)));
                } else {
                    
                    
                    comment.push('-');
                    comment.push('-');
                    pos += 2;
                }
            } else if chars[pos] == '-' {
                comment.push('-');
                pos += 1;
            } else {
                comment.push(chars[pos]);
                pos += 1;
            }
        }

        
        let consumed = pos - start;
        Ok(Some((HtmlNode::Comment(comment), consumed)))
    }

    fn parse_doctype(&mut self, chars: &[char], start: usize) -> Result<Option<(HtmlNode, usize)>, String> {
    let mut pos = start;
    while pos < chars.len() && chars[pos].is_whitespace() {
        pos += 1;
    }

    let mut doctype_word = String::new();
    while pos < chars.len() && !chars[pos].is_whitespace() && chars[pos] != '>' {
        doctype_word.push(chars[pos]);
        pos += 1;
    }

    if doctype_word.to_lowercase() != "doctype" {
        while pos < chars.len() && chars[pos] != '>' {
            pos += 1;
        }
        if pos < chars.len() {
            pos += 1; // '>'
        }
        Ok(Some((HtmlNode::Comment(format!("Invalid DOCTYPE: {}", doctype_word)), pos - start)));
    }
 
    while pos < chars.len() && chars[pos].is_whitespace() {
        pos += 1;
    }

    let mut doc_name = String::new();
    while pos < chars.len() && !chars[pos].is_whitespace() && chars[pos] != '>' {
        doc_name.push(chars[pos]);
        pos += 1;
    }

    if doc_name.is_empty() {
        doc_name = "html".to_string();
   
    while pos < chars.len() && chars[pos].is_whitespace() {
        pos += 1;
    }

    let mut public_id = None;
    let mut system_id = None;
    let mut remaining = String::new();

    while pos < chars.len() && chars[pos] != '>' {
        remaining.push(chars[pos]);
        pos += 1;
    }
    if pos < chars.len() {
        pos += 1; // '>'
    }

    let remaining_lower = remaining.to_lowercase();
    if remaining_lower.contains("public") || remaining_lower.contains("system") {
        let quote_chars = ['"', '\''];
        let mut quotes_found: Vec<String> = Vec::new();
        let mut current_pos = 0;
        let rem_chars: Vec<char> = remaining.chars().collect();
        while current_pos < rem_chars.len() && quotes_found.len() < 2 {
            if quote_chars.contains(&rem_chars[current_pos]) {
                let quote = rem_chars[current_pos];
                current_pos += 1;
                let mut s = String::new();
                while current_pos < rem_chars.len() && rem_chars[current_pos] != quote {
                    s.push(rem_chars[current_pos]);
                    current_pos += 1;
                }
                if current_pos < rem_chars.len() {
                    current_pos += 1;
                }
                quotes_found.push(s);
            } else {
                current_pos += 1;
            }
        }
        if !quotes_found.is_empty() {
            if remaining_lower.contains("public") {
                public_id = Some(quotes_found[0].clone());
                if quotes_found.len() > 1 {
                    system_id = Some(quotes_found[1].clone());
                }
            } else {
                system_id = Some(quotes_found[0].clone());
            }
        }
    }

    Ok(Some((HtmlNode::Doctype {
        name: doc_name,
        public_id,
        system_id,
    }, pos - start)))
}

    fn parse_element(&mut self, chars: &[char], start: usize, _parent_tag: &str, depth: usize) -> Result<Option<(HtmlNode, usize)>, String> {
        let mut pos = start;

        
        let mut tag_name = String::new();
        while pos < chars.len() && (chars[pos].is_alphanumeric() || chars[pos] == '-' || chars[pos] == '_' || chars[pos] == ':') {
            tag_name.push(chars[pos]);
            pos += 1;
        }

        if tag_name.is_empty() {
            return Ok(None);
        }

        
        let tag_lower = tag_name.to_lowercase();
        if !self.supported_tags.contains(&tag_lower.as_str()) {
            self.log_unsupported(format!("Unsupported HTML tag: <{}>", tag_name));
        }

        
        let (attributes, attr_consumed) = self.parse_attributes(chars, pos);
        pos += attr_consumed;

        
        let mut self_closing = false;
        while pos < chars.len() && chars[pos] != '>' {
            if chars[pos] == '/' {
                self_closing = true;
            }
            pos += 1;
        }

        
        if pos < chars.len() && chars[pos] == '>' {
            pos += 1;
        }

        
        let void_elements = ["area", "base", "br", "col", "embed", "hr", "img", "input",
                            "link", "meta", "param", "source", "track", "wbr"];

        let tag_lower = tag_name.to_lowercase();

        
        if self_closing || void_elements.contains(&tag_lower.as_str()) {
            self.node_count += 1;
            let consumed = pos - start;
            return Ok(Some((HtmlNode::Element {
                tag: tag_name,
                attributes,
                children: Vec::new(),
            }, consumed)));
        }

        
        
        let auto_close_tags = ["p", "li", "td", "th", "tr", "option", "dt", "dd", "rb", "rp", "rt", "rtc"];
        let is_auto_close = auto_close_tags.contains(&tag_lower.as_str());

        
        let children = self.parse_children(chars, pos, &tag_name, depth + 1, is_auto_close)?;
        let children_consumed = children.1;
        let total_consumed = (pos - start) + children_consumed;

        self.node_count += 1;
        Ok(Some((HtmlNode::Element {
            tag: tag_name,
            attributes,
            children: children.0,
        }, total_consumed)))
    }

    fn parse_children(&mut self, chars: &[char], start: usize, parent_tag: &str, depth: usize, auto_close: bool) -> Result<(Vec<HtmlNode>, usize), String> {
        let mut children = Vec::new();
        let mut pos = start;
        let parent_lower = parent_tag.to_lowercase();
        let mut consecutive_no_progress = 0;
        const MAX_NO_PROGRESS: usize = 10;

        while pos < chars.len() {
            
            if depth > MAX_DEPTH {
                warn!("parse_children: max depth exceeded at pos {}", pos);
                break;
            }

            
            if self.node_count >= MAX_NODES {
                warn!("parse_children: max nodes limit reached");
                break;
            }

            
            while pos < chars.len() && chars[pos].is_whitespace() {
                pos += 1;
            }

            if pos >= chars.len() {
                break;
            }

            
            if chars[pos] == '<' && pos + 1 < chars.len() && chars[pos + 1] == '/' {
                
                let mut end_tag = String::new();
                let mut end_pos = pos + 2;
                while end_pos < chars.len() && chars[end_pos] != '>' {
                    end_tag.push(chars[end_pos]);
                    end_pos += 1;
                }

                
                if end_pos < chars.len() && chars[end_pos] == '>' {
                    end_pos += 1;
                }

                if end_tag.to_lowercase() == parent_lower {
                    
                    pos = end_pos;
                    break;
                } else {
                    
                    warn!("Mismatched closing tag: expected </{}>, found </{}>", parent_tag, end_tag);
                    pos = end_pos;
                    continue;
                }
            }

            
            let pos_before = pos;

            
            
            if auto_close && chars[pos] == '<' {
                let next_char = chars.get(pos + 1);
                if next_char.map(|c| c.is_alphabetic()).unwrap_or(false) {
                    
                    break;
                }
            }

            match self.parse_node(chars, pos, parent_tag, depth)? {
                Some((node, consumed)) => {
                    if consumed == 0 {
                        consecutive_no_progress += 1;
                        if consecutive_no_progress > MAX_NO_PROGRESS {
                            warn!("parse_children: stuck in loop, breaking at pos {}", pos);
                            break;
                        }
                        pos += 1; 
                    } else {
                        consecutive_no_progress = 0;
                        children.push(node);
                        pos += consumed;
                    }
                }
                None => {
                    
                    consecutive_no_progress += 1;
                    if consecutive_no_progress > MAX_NO_PROGRESS {
                        warn!("parse_children: too many None returns, breaking at pos {}", pos);
                        break;
                    }
                    pos += 1;
                }
            }

            
            if pos <= pos_before {
                consecutive_no_progress += 1;
                if consecutive_no_progress > MAX_NO_PROGRESS {
                    warn!("parse_children: position not advancing, breaking");
                    break;
                }
                pos = pos_before + 1;
            }
        }

        let consumed = pos - start;
        Ok((children, consumed))
    }

    fn parse_attributes(&mut self, chars: &[char], start: usize) -> (HashMap<String, String>, usize) {
        let mut attributes = HashMap::new();
        let mut pos = start;

        while pos < chars.len() {
            
            while pos < chars.len() && chars[pos].is_whitespace() {
                pos += 1;
            }

            if pos >= chars.len() || chars[pos] == '>' || chars[pos] == '/' {
                break;
            }

            
            let mut name = String::new();
            while pos < chars.len() && (chars[pos].is_alphanumeric() || chars[pos] == '-' || chars[pos] == '_' || chars[pos] == ':') {
                name.push(chars[pos]);
                pos += 1;
            }

            if name.is_empty() {
                break;
            }

            
            while pos < chars.len() && chars[pos].is_whitespace() {
                pos += 1;
            }

            let value = if pos < chars.len() && chars[pos] == '=' {
                pos += 1; 
                
                
                while pos < chars.len() && chars[pos].is_whitespace() {
                    pos += 1;
                }

                if pos < chars.len() && (chars[pos] == '"' || chars[pos] == '\'') {
                    let quote = chars[pos];
                    pos += 1;
                    let mut val = String::new();
                    let mut closed = false;
                    while pos < chars.len() {
                        if chars[pos] == quote {
                            closed = true;
                            break;
                        }
                        val.push(chars[pos]);
                        pos += 1;
                    }
                    if !closed {
                        self.log_unsupported(format!("Unclosed attribute quote for '{}'", name));
                    }
                    if pos < chars.len() {
                        pos += 1; 
                    }
                    val
                } else {
                    let mut val = String::new();
                    while pos < chars.len() && !chars[pos].is_whitespace() && chars[pos] != '>' && chars[pos] != '/' {
                        val.push(chars[pos]);
                        pos += 1;
                    }
                    val
                }
            } else {
                String::new()
            };

            
            let is_supported = self.supported_attributes.iter().any(|&attr| {
                if attr.ends_with("*") {
                    name.starts_with(&attr[..attr.len()-1])
                } else {
                    name == attr
                }
            });

            if !is_supported {
                self.log_unsupported(format!("Unsupported attribute: {}=\"{}\"", name, value));
            }

            attributes.insert(name, value);
        }

        let consumed = pos - start;
        (attributes, consumed)
    }

    fn parse_text(&mut self, chars: &[char], start: usize) -> Result<Option<(HtmlNode, usize)>, String> {
        let mut text = String::new();
        let mut pos = start;

        while pos < chars.len() && chars[pos] != '<' {
            text.push(chars[pos]);
            pos += 1;
        }

        
        
        if text.is_empty() {
            return Ok(None);
        }

        
        if text == ">" || text == "</" || text.starts_with('>') || text.ends_with('>') {
            return Ok(None);
        }

        
        if text.chars().all(|c| c == '>' || c == '<' || c == '/' || c == '"' || c == '\'' || c == '=' || c.is_whitespace()) {
            return Ok(None);
        }

        
        if text.contains("{{") || text.contains("{%") {
            self.log_unsupported(format!("Template syntax detected: {}", text));
        }

        let consumed = pos - start;
        Ok(Some((HtmlNode::Text(text), consumed)))
    }

    fn log_unsupported(&mut self, pattern: String) {
        if !self.unsupported_patterns.contains(&pattern) {
            warn!("⚠️  Unsupported: {}", pattern);
            self.unsupported_patterns.push(pattern);
        }
    }

    
    pub fn print_unsupported(&self) {
        if self.unsupported_patterns.is_empty() {
            println!("✅ HTML Parser: All patterns supported");
            return;
        }

        
        let mut categories: std::collections::HashMap<&str, Vec<&String>> = std::collections::HashMap::new();
        for pattern in &self.unsupported_patterns {
            let category = if pattern.starts_with("Unsupported HTML tag:") {
                "Unsupported tags"
            } else if pattern.starts_with("Unsupported attribute:") {
                "Unsupported attributes"
            } else if pattern.starts_with("Template syntax") {
                "Template syntax"
            } else if pattern.starts_with("Unclosed attribute") {
                "Malformed attributes"
            } else if pattern.starts_with("Unknown declaration:") {
                "Unknown declarations"
            } else {
                "Other"
            };
            categories.entry(category).or_default().push(pattern);
        }

        println!("\n🔴 HTML Parser - Unsupported Patterns Summary:");
        println!("═══════════════════════════════════════════");

        let mut stdout = io::stdout();
        for (category, patterns) in &categories {
            writeln!(stdout, "\n  {} ({} unique):", category, patterns.len()).ok();
            
            for (i, pattern) in patterns.iter().take(5).enumerate() {
                writeln!(stdout, "    [{}] {}", i + 1, pattern).ok();
            }
            if patterns.len() > 5 {
                writeln!(stdout, "    ... and {} more", patterns.len() - 5).ok();
            }
        }

        writeln!(stdout, "\nTotal: {} unique unsupported pattern(s)\n", self.unsupported_patterns.len()).ok();
        stdout.flush().ok();
    }

    
    pub fn get_unsupported(&self) -> &[String] {
        &self.unsupported_patterns
    }
}

impl Default for HtmlParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_html() {
        let mut parser = HtmlParser::new();
        let html = r#"<!DOCTYPE html>
<html>
<head><title>Test</title></head>
<body>
    <div class="container">
        <h1>Hello</h1>
        <p>World</p>
    </div>
</body>
</html>"#;

        let nodes = parser.parse(html).unwrap();
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_unsupported_tag() {
        let mut parser = HtmlParser::new();
        let html = r#"<custom-element>Content</custom-element>"#;

        let _ = parser.parse(html);
        assert!(!parser.get_unsupported().is_empty());
    }

    #[test]
    fn test_empty_html() {
        let mut parser = HtmlParser::new();
        let nodes = parser.parse("").unwrap();
        assert!(nodes.is_empty());
    }

    #[test]
    fn test_whitespace_only() {
        let mut parser = HtmlParser::new();
        let nodes = parser.parse("   \n\t  ").unwrap();
        assert!(nodes.is_empty());
    }

    #[test]
    fn test_deep_nesting() {
        let mut parser = HtmlParser::new();
        
        let mut html = String::new();
        for _ in 0..150 {
            html.push_str("<div>");
        }
        html.push_str("Content");
        for _ in 0..150 {
            html.push_str("</div>");
        }
        
        let result = parser.parse(&html);
        
        assert!(result.is_ok() || result.unwrap_err().contains("depth"));
    }

    #[test]
    fn test_malformed_html() {
        let mut parser = HtmlParser::new();
        let html = r#"<div><p>Unclosed paragraph<div>Content</div>"#;
        let nodes = parser.parse(html).unwrap();
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_unclosed_tags() {
        let mut parser = HtmlParser::new();
        let html = r#"<div><p><span>Text"#;
        let nodes = parser.parse(html).unwrap();
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_self_closing_tags() {
        let mut parser = HtmlParser::new();
        let html = r#"<div><br><img src="test.jpg"><hr></div>"#;
        let nodes = parser.parse(html).unwrap();
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_special_characters() {
        let mut parser = HtmlParser::new();
        let html = r#"<div>&lt;script&gt;alert('XSS')&lt;/script&gt;</div>"#;
        let nodes = parser.parse(html).unwrap();
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_large_html() {
        let mut parser = HtmlParser::new();
        
        let mut html = String::new();
        for i in 0..1000 {
            html.push_str(&format!("<p>Paragraph {}</p>", i));
        }
        let nodes = parser.parse(&html).unwrap();
        assert!(nodes.len() > 0);
    }

    #[test]
    fn test_mismatched_closing_tags() {
        let mut parser = HtmlParser::new();
        let html = r#"<div><p>Text</div></p>"#;
        let nodes = parser.parse(html).unwrap();
        assert!(!nodes.is_empty());
    }

    

    #[test]
    fn test_self_closing_non_void_element() {
        
        let mut parser = HtmlParser::new();
        let html = r#"<div />"#;
        let nodes = parser.parse(html).unwrap();
        assert_eq!(nodes.len(), 1);
        if let HtmlNode::Element { children, .. } = &nodes[0] {
            assert!(children.is_empty());
        } else {
            panic!("Expected element node");
        }
    }

    #[test]
    fn test_comment_with_double_dashes() {
        let mut parser = HtmlParser::new();
        let html = r#"<!-- comment -- with -- dashes -->"#;
        let nodes = parser.parse(html).unwrap();
        assert_eq!(nodes.len(), 1);
        if let HtmlNode::Comment(text) = &nodes[0] {
            assert!(text.contains("--"));
        } else {
            panic!("Expected comment node");
        }
    }

    #[test]
    fn test_doctype_with_public_system_id() {
        let mut parser = HtmlParser::new();
        let html = r#"<!DOCTYPE html PUBLIC "-
        let nodes = parser.parse(html).unwrap();
        assert_eq!(nodes.len(), 1);
        if let HtmlNode::Doctype { name, public_id, system_id } = &nodes[0] {
            assert_eq!(name.to_lowercase(), "html");
            assert!(public_id.is_some());
            assert!(system_id.is_some());
        } else {
            panic!("Expected doctype node");
        }
    }

    #[test]
    fn test_unpaired_quotes_in_attributes() {
        
        let mut parser = HtmlParser::new();
        let html = r#"<div class="test>content</div>"#;
        let result = parser.parse(html);
        assert!(result.is_ok());
    }

    #[test]
    fn test_nested_same_name_tags() {
        
        let mut parser = HtmlParser::new();
        let html = r#"<p>Paragraph <p>nested</p>"#;
        let nodes = parser.parse(html).unwrap();
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_angle_brackets_in_text() {
        
        let mut parser = HtmlParser::new();
        let html = r#"<div>2 < 3</div>"#;
        let nodes = parser.parse(html).unwrap();
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_auto_close_p_tag() {
        
        let mut parser = HtmlParser::new();
        let html = r#"<p>First<p>Second</p>"#;
        let nodes = parser.parse(html).unwrap();
        assert!(!nodes.is_empty());
    }
}
