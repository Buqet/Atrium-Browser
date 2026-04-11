




use std::borrow::Cow;
use crate::css::value::{CssValue, Color, CssLength};
use crate::css::selector::Selector;
use thiserror::Error;


#[derive(Clone, Debug)]
pub struct Declaration {
    pub property: String,
    pub value: CssValue,
    pub important: bool,
}


#[derive(Clone, Debug)]
pub struct CssRule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}


#[derive(Clone, Debug)]
pub struct MediaQuery {
    pub feature: String,
    pub value: f32,
    pub unit: String,
}


#[derive(Clone, Debug)]
pub struct MediaRule {
    pub query: String,
    pub conditions: Vec<MediaQuery>,
    pub rules: Vec<CssRule>,
}


#[derive(Clone, Debug)]
pub struct KeyframeStep {
    pub offset: f32,
    pub declarations: Vec<Declaration>,
}


#[derive(Clone, Debug)]
pub struct KeyframesRule {
    pub name: String,
    pub steps: Vec<KeyframeStep>,
}


#[derive(Clone, Debug)]
pub struct CssTransition {
    pub property: String,
    pub duration: f32,
    pub timing_function: String,
    pub delay: f32,
}


#[derive(Clone, Debug)]
pub struct ImportRule {
    pub url: String,
    pub media_query: Option<String>,
    pub supports_query: Option<String>,
}


#[derive(Clone, Debug)]
pub struct SupportsRule {
    pub condition: String,
    pub rules: Vec<CssRule>,
}


#[derive(Clone, Debug, Default)]
pub struct Stylesheet {
    pub rules: Vec<CssRule>,
    pub media_rules: Vec<MediaRule>,
    pub keyframes: Vec<KeyframesRule>,
    pub transitions: Vec<CssTransition>,
    pub imports: Vec<ImportRule>,
    pub supports_rules: Vec<SupportsRule>,
}


#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Unexpected character '{0}' at position {1}")]
    UnexpectedChar(char, usize),

    #[error("Expected '{0}' at position {1}")]
    ExpectedChar(char, usize),

    #[error("Unexpected end of input at position {0}")]
    UnexpectedEof(usize),

    #[error("Invalid selector at position {0}")]
    InvalidSelector(usize),

    #[error("Invalid declaration at position {0}")]
    InvalidDeclaration(usize),

    #[error("Unknown at-rule '{0}' at position {1}")]
    UnknownAtRule(String, usize),

    #[error("Recursion depth exceeded at position {0}")]
    RecursionLimitExceeded(usize),
}



const MAX_RECURSION_DEPTH: usize = 128;


pub struct CssParser {
    pos: usize,
    chars: Vec<char>,
    recursion_depth: usize,
}

impl CssParser {
    pub fn new() -> Self {
        Self {
            pos: 0,
            chars: Vec::new(),
            recursion_depth: 0,
        }
    }

    
    pub fn parse(&mut self, css: &str) -> Result<Stylesheet, ParseError> {
        self.chars = css.chars().collect();
        self.pos = 0;

        let mut stylesheet = Stylesheet::default();

        while !self.is_eof() {
            self.skip_whitespace();

            if self.is_eof() {
                break;
            }

            
            if self.peek() == Some('@') {
                self.parse_at_rule(&mut stylesheet)?;
                continue;
            }

            
            if let Some(rule) = self.parse_rule()? {
                stylesheet.rules.push(rule);
            } else {
                
                if !self.is_eof() {
                    self.advance();
                }
            }
        }

        Ok(stylesheet)
    }

    fn parse_at_rule(&mut self, stylesheet: &mut Stylesheet) -> Result<(), ParseError> {
        self.advance(); 

        let name = self.parse_identifier()
            .ok_or_else(|| ParseError::UnexpectedEof(self.pos))?;

        match name.as_str() {
            "media" => {
                if let Some(media_rule) = self.parse_media_rule_inner()? {
                    stylesheet.media_rules.push(media_rule);
                }
            }
            "import" => {
                if let Some(import) = self.parse_import_rule_inner()? {
                    stylesheet.imports.push(import);
                }
            }
            "supports" => {
                if let Some(supports) = self.parse_supports_rule_inner()? {
                    stylesheet.supports_rules.push(supports);
                }
            }
            "keyframes" | "-webkit-keyframes" | "-moz-keyframes" | "-o-keyframes" => {
                if let Some(keyframes) = self.parse_keyframes_inner()? {
                    stylesheet.keyframes.push(keyframes);
                }
            }
            "charset" | "namespace" | "font-face" | "page" | "counter-style" => {
                
                self.skip_at_rule_body();
            }
            _ => {
                
                let start_pos = self.pos;
                self.skip_at_rule_body();
                
                
            }
        }

        Ok(())
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek();
        self.pos += 1;
        c
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else if c == '/' && self.chars.get(self.pos + 1) == Some(&'*') {
                
                self.pos += 2;
                while self.pos + 1 < self.chars.len() {
                    if self.chars[self.pos] == '*' && self.chars[self.pos + 1] == '/' {
                        self.pos += 2;
                        break;
                    }
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    
    fn skip_at_rule_body(&mut self) {
        
        while let Some(c) = self.advance() {
            if c == ';' {
                return;
            }
            if c == '{' {
                let mut depth = 1;
                while depth > 0 {
                    match self.advance() {
                        Some('{') => depth += 1,
                        Some('}') => depth -= 1,
                        None => return,
                        _ => {}
                    }
                }
                return;
            }
        }
    }

    fn parse_media_rule_inner(&mut self) -> Result<Option<MediaRule>, ParseError> {
        self.skip_whitespace();

        let query = self.parse_media_query()
            .ok_or_else(|| ParseError::UnexpectedEof(self.pos))?;

        self.skip_whitespace();

        if self.advance() != Some('{') {
            return Err(ParseError::ExpectedChar('{', self.pos));
        }

        let mut rules = Vec::new();
        while !self.is_eof() {
            self.skip_whitespace();

            if self.peek() == Some('}') {
                self.advance();
                break;
            }

            if let Some(rule) = self.parse_rule()? {
                rules.push(rule);
            }
        }

        let conditions = Self::parse_media_conditions(&query);

        Ok(Some(MediaRule {
            query,
            conditions,
            rules,
        }))
    }

    fn parse_import_rule_inner(&mut self) -> Result<Option<ImportRule>, ParseError> {
        self.skip_whitespace();

        let mut url = String::new();
        let mut media_query: Option<String> = None;

        
        if let Some(c) = self.peek() {
            if c == '"' || c == '\'' {
                let quote = c;
                self.advance();
                while let Some(c) = self.peek() {
                    if c == quote {
                        self.advance();
                        break;
                    }
                    url.push(c);
                    self.advance();
                }
            } else if c == 'u' {
                
                if let Some(id) = self.parse_identifier() {
                    if id == "url" && self.peek() == Some('(') {
                        self.advance();
                        self.skip_whitespace();
                        while let Some(c) = self.peek() {
                            if c == ')' {
                                self.advance();
                                break;
                            }
                            url.push(c);
                            self.advance();
                        }
                        url = url.trim_matches('"').trim_matches('\'').to_string();
                    }
                }
            }
        }

        self.skip_whitespace();

        
        if self.peek().map(|c| c.is_alphabetic()).unwrap_or(false) {
            let mut mq = String::new();
            while let Some(c) = self.peek() {
                if c == ';' {
                    break;
                }
                mq.push(c);
                self.advance();
            }
            if !mq.trim().is_empty() {
                media_query = Some(mq.trim().to_string());
            }
        }

        if self.peek() == Some(';') {
            self.advance();
        }

        Ok(Some(ImportRule {
            url: url.trim().to_string(),
            media_query,
            supports_query: None,
        }))
    }

    fn parse_supports_rule_inner(&mut self) -> Result<Option<SupportsRule>, ParseError> {
        self.skip_whitespace();

        let mut condition = String::new();
        while let Some(c) = self.peek() {
            if c == '{' {
                break;
            }
            condition.push(c);
            self.advance();
        }

        self.skip_whitespace();

        if self.advance() != Some('{') {
            return Err(ParseError::ExpectedChar('{', self.pos));
        }

        let mut rules = Vec::new();
        while !self.is_eof() {
            self.skip_whitespace();

            if self.peek() == Some('}') {
                self.advance();
                break;
            }

            if let Some(rule) = self.parse_rule()? {
                rules.push(rule);
            }
        }

        Ok(Some(SupportsRule {
            condition: condition.trim().to_string(),
            rules,
        }))
    }

    fn parse_keyframes_inner(&mut self) -> Result<Option<KeyframesRule>, ParseError> {
        self.skip_whitespace();

        let name = self.parse_identifier()
            .ok_or_else(|| ParseError::UnexpectedEof(self.pos))?;

        self.skip_whitespace();

        if self.advance() != Some('{') {
            return Err(ParseError::ExpectedChar('{', self.pos));
        }

        let mut steps = Vec::new();
        while !self.is_eof() {
            self.skip_whitespace();

            if self.peek() == Some('}') {
                self.advance();
                break;
            }

            if let Some(step) = self.parse_keyframe_step()? {
                steps.push(step);
            }
        }

        Ok(Some(KeyframesRule { name, steps }))
    }

    fn parse_keyframe_step(&mut self) -> Result<Option<KeyframeStep>, ParseError> {
        self.skip_whitespace();

        
        let mut offsets = Vec::new();
        loop {
            let mut offset_str = String::new();
            while let Some(c) = self.peek() {
                if c == '{' || c == ',' {
                    break;
                }
                offset_str.push(c);
                self.advance();
            }

            let offset_str = offset_str.trim().to_lowercase();
            let offset = match offset_str.as_str() {
                "from" => 0.0,
                "to" => 1.0,
                _ => {
                    let pct = offset_str.trim_end_matches('%');
                    pct.parse::<f32>().ok().map(|v| v / 100.0).unwrap_or(0.0)
                }
            };
            offsets.push(offset);

            self.skip_whitespace();
            if self.peek() == Some(',') {
                self.advance();
                self.skip_whitespace();
            } else {
                break;
            }
        }

        if self.advance() != Some('{') {
            return Ok(None);
        }

        let declarations = self.parse_declarations();

        self.skip_whitespace();
        if self.advance() != Some('}') {
            return Ok(None);
        }

        
        
        let offset = offsets.first().copied().unwrap_or(0.0);

        Ok(Some(KeyframeStep { offset, declarations }))
    }

    fn parse_media_query(&mut self) -> Option<String> {
        let mut query = String::new();
        let mut paren_depth = 0;

        while let Some(c) = self.peek() {
            if c == '{' && paren_depth == 0 {
                break;
            }
            if c == '(' {
                paren_depth += 1;
            } else if c == ')' {
                paren_depth -= 1;
            }
            query.push(c);
            self.advance();
        }

        let query = query.trim().to_string();
        if query.is_empty() {
            None
        } else {
            Some(query)
        }
    }

    fn parse_media_conditions(query: &str) -> Vec<MediaQuery> {
        let mut conditions = Vec::new();

        let mut chars = query.chars().peekable();
        let mut current = String::new();
        let mut paren_depth = 0;

        while let Some(c) = chars.next() {
            if c == '(' {
                paren_depth += 1;
                if paren_depth == 1 {
                    current.clear();
                    continue;
                }
            } else if c == ')' {
                paren_depth -= 1;
                if paren_depth == 0 {
                    if let Some(cond) = Self::parse_single_condition(&current) {
                        conditions.push(cond);
                    }
                    current.clear();
                    continue;
                }
            }

            if paren_depth >= 1 {
                current.push(c);
            }
        }

        conditions
    }

    fn parse_single_condition(cond_str: &str) -> Option<MediaQuery> {
        let parts: Vec<&str> = cond_str.split(':').collect();
        if parts.len() != 2 {
            return None;
        }

        let feature = parts[0].trim().to_string();
        let value_str = parts[1].trim();

        let mut num_str = String::new();
        let mut unit = String::new();

        for c in value_str.chars() {
            if c.is_digit(10) || c == '.' || c == '-' {
                num_str.push(c);
            } else if !c.is_whitespace() {
                unit.push(c);
            }
        }

        if let Ok(value) = num_str.parse::<f32>() {
            Some(MediaQuery {
                feature,
                value,
                unit,
            })
        } else {
            None
        }
    }

    fn parse_rule(&mut self) -> Result<Option<CssRule>, ParseError> {
        let selectors = match self.parse_selectors() {
            Some(s) => s,
            None => return Ok(None),
        };

        self.skip_whitespace();

        if self.advance() != Some('{') {
            
            return self.recover_from_rule_error();
        }

        let declarations = self.parse_declarations();

        self.skip_whitespace();
        if self.advance() != Some('}') {
            return Err(ParseError::ExpectedChar('}', self.pos));
        }

        Ok(Some(CssRule {
            selectors,
            declarations,
        }))
    }

    
    fn recover_from_rule_error(&mut self) -> Result<Option<CssRule>, ParseError> {
        while let Some(c) = self.advance() {
            if c == '}' {
                return Ok(None);
            }
        }
        Ok(None)
    }

    fn parse_selectors(&mut self) -> Option<Vec<Selector>> {
        let mut selectors = Vec::new();

        loop {
            self.skip_whitespace();

            if self.recursion_depth >= MAX_RECURSION_DEPTH {
                
                return if selectors.is_empty() { None } else { Some(selectors) };
            }

            if let Some(selector) = self.parse_selector(0) {
                selectors.push(selector);
            }

            self.skip_whitespace();

            match self.peek() {
                Some(',') => {
                    self.advance();
                }
                Some('{') => {
                    break;
                }
                _ => {
                    if self.peek().map(|c| !c.is_whitespace()).unwrap_or(false) {
                        
                        
                        self.advance();
                        continue;
                    }
                    break;
                }
            }
        }

        if selectors.is_empty() {
            None
        } else {
            Some(selectors)
        }
    }

    fn parse_selector(&mut self, depth: usize) -> Option<Selector> {
        if depth >= MAX_RECURSION_DEPTH {
            return None;
        }

        self.skip_whitespace();

        let mut selector = self.parse_simple_selector()?;

        loop {
            let before_space = self.pos;
            self.skip_whitespace();
            let had_space = self.pos > before_space;

            if self.is_eof() {
                break;
            }

            match self.peek() {
                Some('>') => {
                    self.advance();
                    self.skip_whitespace();
                    if let Some(right) = self.parse_simple_selector() {
                        selector = Selector::Child(Box::new(selector), Box::new(right));
                    }
                }
                Some('+') => {
                    self.advance();
                    self.skip_whitespace();
                    if let Some(right) = self.parse_simple_selector() {
                        selector = Selector::Adjacent(Box::new(selector), Box::new(right));
                    }
                }
                Some('~') => {
                    self.advance();
                    self.skip_whitespace();
                    if let Some(right) = self.parse_simple_selector() {
                        selector = Selector::GeneralSibling(Box::new(selector), Box::new(right));
                    }
                }
                Some(c) if c.is_alphanumeric() || c == '.' || c == '#' || c == '[' || c == ':' => {
                    if had_space {
                        if let Some(right) = self.parse_simple_selector() {
                            selector = Selector::Descendant(Box::new(selector), Box::new(right));
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }

        Some(selector)
    }

    fn parse_simple_selector(&mut self) -> Option<Selector> {
        self.skip_whitespace();

        match self.peek()? {
            '*' => {
                self.advance();
                Some(Selector::Universal)
            }
            '.' => {
                self.advance();
                Some(Selector::Class(self.parse_identifier()?))
            }
            '#' => {
                self.advance();
                Some(Selector::Id(self.parse_identifier()?))
            }
            '[' => {
                self.advance();
                let attr = self.parse_attribute_selector()?;
                self.skip_whitespace();
                if self.peek() == Some(']') {
                    self.advance();
                }
                Some(attr)
            }
            ':' => {
                self.advance();
                if self.peek() == Some(':') {
                    self.advance();
                    Some(Selector::PseudoElement(self.parse_identifier()?))
                } else {
                    
                    self.parse_functional_pseudo_class()
                }
            }
            c if is_ident_start(c) => {
                Some(Selector::Type(self.parse_identifier()?))
            }
            _ => None,
        }
    }

    fn parse_functional_pseudo_class(&mut self) -> Option<Selector> {
        let name = self.parse_identifier()?;

        self.skip_whitespace();

        if self.peek() != Some('(') {
            return Some(Selector::PseudoClass(name));
        }

        self.advance(); 
        self.skip_whitespace();

        let selector = match name.as_str() {
            "not" => {
                self.skip_whitespace();
                if let Some(inner) = self.parse_simple_selector() {
                    Some(Selector::Not(Box::new(inner)))
                } else {
                    Some(Selector::PseudoClass("not".to_string()))
                }
            }
            "nth-child" => {
                let (a, b) = self.parse_nth_child_arg();
                Some(Selector::NthChild(a, b))
            }
            _ => Some(Selector::PseudoClass(name)),
        };

        self.skip_whitespace();
        if self.peek() == Some(')') {
            self.advance();
        }

        selector
    }

    fn parse_nth_child_arg(&mut self) -> (i32, i32) {
        let mut arg = String::new();
        while let Some(c) = self.peek() {
            if c == ')' {
                break;
            }
            arg.push(c);
            self.advance();
        }

        let arg = arg.trim();

        if arg == "odd" {
            return (2, 1);
        }
        if arg == "even" {
            return (2, 0);
        }

        
        if let Some(n_pos) = arg.find('n') {
            let a_part = arg[..n_pos].trim();
            let a: i32 = if a_part.is_empty() || a_part == "+" {
                1
            } else if a_part == "-" {
                -1
            } else {
                a_part.parse().unwrap_or(1)
            };
            let b_str = arg[n_pos + 1..].trim();
            let b: i32 = if b_str.is_empty() {
                0
            } else {
                b_str.parse().unwrap_or(0)
            };
            (a, b)
        } else if let Ok(n) = arg.parse::<i32>() {
            (0, n)
        } else {
            (0, 0)
        }
    }

    fn parse_identifier(&mut self) -> Option<String> {
        let mut ident = String::new();

        while let Some(c) = self.peek() {
            if is_ident_char(c) {
                ident.push(c);
                self.advance();
            } else {
                break;
            }
        }

        if ident.is_empty() {
            None
        } else {
            Some(ident)
        }
    }

    fn parse_attribute_selector(&mut self) -> Option<Selector> {
        self.skip_whitespace();
        let name = self.parse_identifier()?;

        self.skip_whitespace();

        if self.peek() == Some(']') {
            return Some(Selector::Attribute(name, None));
        }

        
        let op1 = self.advance()?;
        
        let match_type = match op1 {
            '=' => crate::css::matcher::AttributeMatchType::Exact,
            '~' => {
                if self.advance() != Some('=') {
                    return None;
                }
                crate::css::matcher::AttributeMatchType::Includes
            }
            '|' => {
                if self.advance() != Some('=') {
                    return None;
                }
                crate::css::matcher::AttributeMatchType::DashMatch
            }
            '^' => {
                if self.advance() != Some('=') {
                    return None;
                }
                crate::css::matcher::AttributeMatchType::Prefix
            }
            '$' => {
                if self.advance() != Some('=') {
                    return None;
                }
                crate::css::matcher::AttributeMatchType::Suffix
            }
            '*' => {
                if self.advance() != Some('=') {
                    return None;
                }
                crate::css::matcher::AttributeMatchType::Substring
            }
            _ => return None,
        };

        self.skip_whitespace();

        
        let mut value = String::new();
        let quote = if self.peek() == Some('"') || self.peek() == Some('\'') {
            let q = self.advance().unwrap();
            Some(q)
        } else {
            None
        };

        while let Some(c) = self.peek() {
            if Some(c) == quote || (quote.is_none() && (c == ']' || c.is_whitespace())) {
                break;
            }
            value.push(c);
            self.advance();
        }

        if quote.is_some() {
            self.advance(); 
        }

        let value = value.trim().to_string();

        
        let _case_sensitive = true; 
        self.skip_whitespace();
        if let Some(c) = self.peek() {
            if c == 'i' || c == 'I' {
                
                
                self.advance();
            } else if c == 's' || c == 'S' {
                
                self.advance();
            }
        }

        Some(Selector::Attribute(name, Some((value, match_type))))
    }

    fn parse_declarations(&mut self) -> Vec<Declaration> {
        let mut declarations = Vec::new();

        loop {
            self.skip_whitespace();

            if self.peek() == Some('}') {
                break;
            }

            if let Some(decl) = self.parse_declaration() {
                
                let expanded = crate::css::properties::expand_shorthand(&decl.property, &decl.value);
                for (prop, val) in expanded {
                    declarations.push(Declaration {
                        property: prop,
                        value: val,
                        important: decl.important,
                    });
                }
            }

            self.skip_whitespace();
            if self.peek() == Some(';') {
                self.advance();
            }
        }

        declarations
    }

    fn parse_declaration(&mut self) -> Option<Declaration> {
        let property = self.parse_identifier()?;

        self.skip_whitespace();

        if self.advance() != Some(':') {
            return None;
        }

        let (value, important) = self.parse_value_with_important()?;

        Some(Declaration { property, value, important })
    }

    fn parse_value_with_important(&mut self) -> Option<(CssValue, bool)> {
        self.skip_whitespace();

        let mut value = String::new();
        let mut important = false;

        while let Some(c) = self.peek() {
            if c == ';' || c == '}' {
                break;
            }
            if c == '!' {
                let remaining: String = self.chars[self.pos..].iter().collect();
                if remaining.to_lowercase().starts_with("!important") {
                    important = true;
                    self.pos += 10;
                    continue;
                }
            }
            value.push(c);
            self.advance();
        }

        let value = value.trim();

        if value.is_empty() {
            return None;
        }

        let css_value = parse_css_value_string(value);

        css_value.map(|v| (v, important))
    }
}

impl Default for CssParser {
    fn default() -> Self {
        Self::new()
    }
}


fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '-' || ('\u{0080}'..='\u{FFFF}').contains(&c)
}




fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
        || ('\u{0080}'..='\u{FFFF}').contains(&c)
}


pub fn parse_css_value_string(value: &str) -> Option<CssValue> {
    let value = value.trim();

    if value.is_empty() {
        return None;
    }

    
    if value.eq_ignore_ascii_case("none") {
        return Some(CssValue::None);
    }
    if value.eq_ignore_ascii_case("auto") {
        return Some(CssValue::Auto);
    }
    if value.eq_ignore_ascii_case("inherit") {
        return Some(CssValue::Inherit);
    }
    if value.eq_ignore_ascii_case("initial") {
        return Some(CssValue::Initial);
    }
    if value.eq_ignore_ascii_case("unset") {
        return Some(CssValue::Unset);
    }
    if value.eq_ignore_ascii_case("revert") {
        return Some(CssValue::Revert);
    }

    
    if value.starts_with("calc(") {
        
        return Some(CssValue::Keyword(Cow::Owned(value.to_string())));
    }

    
    if value.starts_with("min(") || value.starts_with("max(") || value.starts_with("clamp(") {
        return Some(CssValue::Keyword(Cow::Owned(value.to_string())));
    }

    
    if value.starts_with("var(") {
        return Some(CssValue::String(Cow::Owned(value.to_string())));
    }

    
    if value.starts_with('#') {
        if let Some(color) = Color::from_hex(&value[1..]) {
            return Some(CssValue::Color(color));
        }
    }

    
    if let Some(color) = parse_any_color(value) {
        return Some(CssValue::Color(color));
    }

    if let Some(color) = Color::named(value) {
        return Some(CssValue::Color(color));
    }

    
    if value.starts_with("url(") && value.ends_with(')') {
        let url = value[4..value.len()-1].trim().trim_matches('"').trim_matches('\'');
        return Some(CssValue::Url(Cow::Owned(url.to_string())));
    }

    
    if let Some(len) = parse_length_value(value) {
        return Some(CssValue::Length(len));
    }

    
    if let Ok(num) = value.parse::<f32>() {
        return Some(CssValue::Number(num));
    }

    
    Some(CssValue::Keyword(Cow::Owned(value.to_string())))
}


fn parse_any_color(value: &str) -> Option<Color> {
    let v = value.trim().to_lowercase();
    
    
    if v.starts_with("rgb(") || v.starts_with("rgba(") {
        return parse_rgb_color(&v);
    }
    
    
    if v.starts_with("hsl(") || v.starts_with("hsla(") {
        return parse_hsl_color(&v);
    }
    
    
    if v.starts_with("hwb(") {
        return parse_hwb_color(&v);
    }
    
    
    if v.starts_with("lab(") {
        return parse_lab_color(&v);
    }
    
    
    if v.starts_with("lch(") {
        return parse_lch_color(&v);
    }
    
    
    if v.starts_with("oklab(") {
        return parse_oklab_color(&v);
    }
    
    
    if v.starts_with("oklch(") {
        return parse_oklch_color(&v);
    }
    
    
    if v.starts_with("color(") {
        return parse_color_function(&v);
    }
    
    None
}



fn parse_rgb_color(value: &str) -> Option<Color> {
    let inner = value
        .trim_start_matches("rgba(")
        .trim_start_matches("rgb(")
        .trim_end_matches(')')
        .trim();
    
    
    if let Some(slash_pos) = inner.find('/') {
        let rgb_part = &inner[..slash_pos].trim();
        let alpha_part = inner[slash_pos + 1..].trim();
        
        let parts: Vec<f32> = rgb_part.split_whitespace()
            .filter_map(|p| p.replace('%', "").parse::<f32>().ok())
            .collect();
        
        if parts.len() < 3 { return None; }
        
        let r = if parts[0] > 1.0 { parts[0] as u8 } else { (parts[0] * 255.0) as u8 };
        let g = if parts[1] > 1.0 { parts[1] as u8 } else { (parts[1] * 255.0) as u8 };
        let b = if parts[2] > 1.0 { parts[2] as u8 } else { (parts[2] * 255.0) as u8 };
        let a = (alpha_part.trim_end_matches('%').parse::<f32>().ok()? / 
                 if alpha_part.contains('%') { 100.0 } else { 1.0 }).clamp(0.0, 1.0);
        
        Some(Color::from_rgba(r, g, b, (a * 255.0) as u8))
    } else {
        
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() < 3 { return None; }
        
        let r = parts[0].trim().parse::<u8>().ok()?;
        let g = parts[1].trim().parse::<u8>().ok()?;
        let b = parts[2].trim().parse::<u8>().ok()?;
        let a = if parts.len() >= 4 {
            (parts[3].trim().parse::<f32>().ok()? / 
             if parts[3].trim().contains('%') { 100.0 } else { 1.0 }).clamp(0.0, 1.0)
        } else {
            1.0
        };
        
        Some(Color::from_rgba(r, g, b, (a * 255.0) as u8))
    }
}



fn parse_hsl_color(value: &str) -> Option<Color> {
    let inner = value
        .trim_start_matches("hsla(")
        .trim_start_matches("hsl(")
        .trim_end_matches(')')
        .trim();
    
    let (hue_part, sat_light_alpha) = if let Some(slash_pos) = inner.find('/') {
        (&inner[..slash_pos], Some(inner[slash_pos + 1..].trim()))
    } else {
        (inner, None)
    };
    
    
    let parts: Vec<&str> = if hue_part.contains(',') {
        hue_part.split(',').collect()
    } else {
        hue_part.split_whitespace().collect()
    };
    
    if parts.len() < 3 { return None; }
    
    let h = parts[0].trim().parse::<f32>().ok()?;
    let s = parts[1].trim().trim_end_matches('%').parse::<f32>().ok()?;
    let l = parts[2].trim().trim_end_matches('%').parse::<f32>().ok()?;
    
    let alpha = if let Some(a) = sat_light_alpha {
        (a.trim_end_matches('%').parse::<f32>().ok()? / 
         if a.contains('%') { 100.0 } else { 1.0 }).clamp(0.0, 1.0)
    } else {
        1.0
    };
    
    Some(Color::from_hsl(h, s, l, alpha))
}



fn parse_hwb_color(value: &str) -> Option<Color> {
    let inner = value
        .trim_start_matches("hwb(")
        .trim_end_matches(')')
        .trim();
    
    let (main_part, alpha) = if let Some(slash_pos) = inner.find('/') {
        (&inner[..slash_pos], Some(inner[slash_pos + 1..].trim()))
    } else {
        (inner, None)
    };
    
    let parts: Vec<&str> = main_part.split_whitespace().collect();
    if parts.len() < 3 { return None; }
    
    let h = parts[0].trim().parse::<f32>().ok()?;
    let w = parts[1].trim().trim_end_matches('%').parse::<f32>().ok()?;
    let b = parts[2].trim().trim_end_matches('%').parse::<f32>().ok()?;
    
    let alpha = if let Some(a) = alpha {
        (a.trim_end_matches('%').parse::<f32>().ok()? / 
         if a.contains('%') { 100.0 } else { 1.0 }).clamp(0.0, 1.0)
    } else {
        1.0
    };
    
    Some(Color::from_hwb(h, w, b, alpha))
}



fn parse_lab_color(value: &str) -> Option<Color> {
    let inner = value
        .trim_start_matches("lab(")
        .trim_end_matches(')')
        .trim();
    
    let (main_part, alpha) = if let Some(slash_pos) = inner.find('/') {
        (&inner[..slash_pos], Some(inner[slash_pos + 1..].trim()))
    } else {
        (inner, None)
    };
    
    let parts: Vec<&str> = main_part.split_whitespace().collect();
    if parts.len() < 3 { return None; }
    
    let l = parts[0].trim().trim_end_matches('%').parse::<f32>().ok()?;
    let a = parts[1].trim().parse::<f32>().ok()?;
    let b = parts[2].trim().parse::<f32>().ok()?;
    
    let alpha = if let Some(al) = alpha {
        (al.trim_end_matches('%').parse::<f32>().ok()? / 
         if al.contains('%') { 100.0 } else { 1.0 }).clamp(0.0, 1.0)
    } else {
        1.0
    };
    
    Some(Color::from_lab(l, a, b, alpha))
}



fn parse_lch_color(value: &str) -> Option<Color> {
    let inner = value
        .trim_start_matches("lch(")
        .trim_end_matches(')')
        .trim();
    
    let (main_part, alpha) = if let Some(slash_pos) = inner.find('/') {
        (&inner[..slash_pos], Some(inner[slash_pos + 1..].trim()))
    } else {
        (inner, None)
    };
    
    let parts: Vec<&str> = main_part.split_whitespace().collect();
    if parts.len() < 3 { return None; }
    
    let l = parts[0].trim().trim_end_matches('%').parse::<f32>().ok()?;
    let c = parts[1].trim().parse::<f32>().ok()?;
    let h = parts[2].trim().parse::<f32>().ok()?;
    
    let alpha = if let Some(al) = alpha {
        (al.trim_end_matches('%').parse::<f32>().ok()? / 
         if al.contains('%') { 100.0 } else { 1.0 }).clamp(0.0, 1.0)
    } else {
        1.0
    };
    
    Some(Color::from_lch(l, c, h, alpha))
}



fn parse_oklab_color(value: &str) -> Option<Color> {
    let inner = value
        .trim_start_matches("oklab(")
        .trim_end_matches(')')
        .trim();
    
    let (main_part, alpha) = if let Some(slash_pos) = inner.find('/') {
        (&inner[..slash_pos], Some(inner[slash_pos + 1..].trim()))
    } else {
        (inner, None)
    };
    
    let parts: Vec<&str> = main_part.split_whitespace().collect();
    if parts.len() < 3 { return None; }
    
    let l = parts[0].trim().parse::<f32>().ok()?;
    let a = parts[1].trim().parse::<f32>().ok()?;
    let b = parts[2].trim().parse::<f32>().ok()?;
    
    let alpha = if let Some(al) = alpha {
        (al.trim_end_matches('%').parse::<f32>().ok()? / 
         if al.contains('%') { 100.0 } else { 1.0 }).clamp(0.0, 1.0)
    } else {
        1.0
    };
    
    Some(Color::from_oklab(l, a, b, alpha))
}



fn parse_oklch_color(value: &str) -> Option<Color> {
    let inner = value
        .trim_start_matches("oklch(")
        .trim_end_matches(')')
        .trim();
    
    let (main_part, alpha) = if let Some(slash_pos) = inner.find('/') {
        (&inner[..slash_pos], Some(inner[slash_pos + 1..].trim()))
    } else {
        (inner, None)
    };
    
    let parts: Vec<&str> = main_part.split_whitespace().collect();
    if parts.len() < 3 { return None; }
    
    let l = parts[0].trim().parse::<f32>().ok()?;
    let c = parts[1].trim().parse::<f32>().ok()?;
    let h = parts[2].trim().parse::<f32>().ok()?;
    
    let alpha = if let Some(al) = alpha {
        (al.trim_end_matches('%').parse::<f32>().ok()? / 
         if al.contains('%') { 100.0 } else { 1.0 }).clamp(0.0, 1.0)
    } else {
        1.0
    };
    
    Some(Color::from_oklch(l, c, h, alpha))
}



fn parse_color_function(value: &str) -> Option<Color> {
    let inner = value
        .trim_start_matches("color(")
        .trim_end_matches(')')
        .trim();
    
    let (main_part, alpha) = if let Some(slash_pos) = inner.find('/') {
        (&inner[..slash_pos], Some(inner[slash_pos + 1..].trim()))
    } else {
        (inner, None)
    };
    
    let parts: Vec<&str> = main_part.split_whitespace().collect();
    if parts.len() < 3 { return None; }
    
    let color_space = parts[0];
    let components: Vec<f32> = parts[1..].iter()
        .filter_map(|p| p.trim().parse::<f32>().ok())
        .collect();
    
    if components.len() < 3 { return None; }
    
    let alpha = if let Some(al) = alpha {
        (al.trim_end_matches('%').parse::<f32>().ok()? / 
         if al.contains('%') { 100.0 } else { 1.0 }).clamp(0.0, 1.0)
    } else {
        1.0
    };
    
    Some(Color::from_color_space(color_space, &components, alpha))
}


fn parse_length_value(value: &str) -> Option<CssLength> {
    let value = value.trim();

    let mut num_str = String::new();
    let mut unit = String::new();

    for c in value.chars() {
        if c.is_digit(10) || c == '.' || c == '-' || c == '+' {
            num_str.push(c);
        } else if !c.is_whitespace() {
            unit.push(c);
        }
    }

    let num: f32 = num_str.parse().ok()?;

    Some(CssLength::from_value_and_unit(num, &unit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_rule() {
        let mut parser = CssParser::new();
        let result = parser.parse("div { color: red; }");
        assert!(result.is_ok());
        let stylesheet = result.unwrap();
        assert_eq!(stylesheet.rules.len(), 1);
        assert_eq!(stylesheet.rules[0].declarations.len(), 1);
    }

    #[test]
    fn test_parse_media_query() {
        let mut parser = CssParser::new();
        let result = parser.parse("@media (max-width: 600px) { div { color: blue; } }");
        assert!(result.is_ok());
        let stylesheet = result.unwrap();
        assert_eq!(stylesheet.media_rules.len(), 1);
    }

    #[test]
    fn test_parse_important() {
        let mut parser = CssParser::new();
        let result = parser.parse("div { color: red !important; }");
        assert!(result.is_ok());
        let stylesheet = result.unwrap();
        assert!(stylesheet.rules[0].declarations[0].important);
    }

    #[test]
    fn test_parse_nth_child() {
        let mut parser = CssParser::new();
        let result = parser.parse("tr:nth-child(2n+1) { background: gray; }");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_not() {
        let mut parser = CssParser::new();
        let result = parser.parse("div:not(.special) { color: blue; }");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_import() {
        let mut parser = CssParser::new();
        let result = parser.parse("@import url('styles.css');");
        assert!(result.is_ok());
        let stylesheet = result.unwrap();
        assert_eq!(stylesheet.imports.len(), 1);
    }

    #[test]
    fn test_parse_supports() {
        let mut parser = CssParser::new();
        let result = parser.parse("@supports (display: grid) { div { display: grid; } }");
        assert!(result.is_ok());
        let stylesheet = result.unwrap();
        assert_eq!(stylesheet.supports_rules.len(), 1);
    }

    #[test]
    fn test_error_recovery() {
        let mut parser = CssParser::new();
        
        let result = parser.parse("div { color: ; } p { color: red; }");
        assert!(result.is_ok());
    }

    #[test]
    fn test_length_parsing() {
        assert_eq!(parse_length_value("10px"), Some(CssLength::Px(10.0)));
        assert_eq!(parse_length_value("2em"), Some(CssLength::Em(2.0)));
        assert_eq!(parse_length_value("50%"), Some(CssLength::Percent(50.0)));
        assert_eq!(parse_length_value("100vw"), Some(CssLength::Vw(100.0)));
        assert_eq!(parse_length_value("100vh"), Some(CssLength::Vh(100.0)));
    }

    #[test]
    fn test_css_value_keywords() {
        assert!(matches!(parse_css_value_string("none"), Some(CssValue::None)));
        assert!(matches!(parse_css_value_string("auto"), Some(CssValue::Auto)));
        assert!(matches!(parse_css_value_string("inherit"), Some(CssValue::Inherit)));
        assert!(matches!(parse_css_value_string("initial"), Some(CssValue::Initial)));
        assert!(matches!(parse_css_value_string("unset"), Some(CssValue::Unset)));
        assert!(matches!(parse_css_value_string("revert"), Some(CssValue::Revert)));
    }
}
