use std::collections::HashMap;
use crate::css::value::{CssValue, Color};


#[derive(Clone, Debug, Default)]
pub struct CustomProperties {
    properties: HashMap<String, CssValue>,
}

impl CustomProperties {
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
        }
    }

    
    pub fn set(&mut self, name: &str, value: CssValue) {
        self.properties.insert(name.to_string(), value);
    }

    
    pub fn get(&self, name: &str) -> Option<&CssValue> {
        self.properties.get(name)
    }

    
    pub fn resolve_var(&self, value: &str) -> String {
        let mut result = value.to_string();

        while let Some(start) = result.find("var(") {
            if let Some(end) = result[start..].find(')') {
                let end = start + end;
                let var_expr = &result[start + 4..end];

                let parts: Vec<&str> = var_expr.splitn(2, ',').collect();
                let var_name = parts[0].trim();
                let fallback = if parts.len() > 1 { parts[1].trim() } else { "" };

                let replacement = if let Some(css_value) = self.get(var_name) {
                    match css_value {
                        CssValue::String(s) => s.to_string(),
                        CssValue::Number(n) => n.to_string(),
                        CssValue::Keyword(k) => k.to_string(),
                        CssValue::Color(c) => format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b),
                        CssValue::Url(u) => format!("url({})", u),
                        CssValue::Length(len) => format!("{}px", len.value()),
                        _ => String::new(),
                    }
                } else if !fallback.is_empty() {
                    fallback.to_string()
                } else {
                    String::new()
                };

                result = format!("{}{}{}", &result[..start], replacement, &result[end + 1..]);
            } else {
                break;
            }
        }

        result
    }

    
    pub fn extract_from_rules(&mut self, rules: &[super::parser::CssRule]) {
        for rule in rules {
            for selector in &rule.selectors {
                let is_root = matches!(selector, super::selector::Selector::Type(t) if t == "html" || t == ":root")
                    || matches!(selector, super::selector::Selector::Universal);

                if is_root {
                    for decl in &rule.declarations {
                        if decl.property.starts_with("--") {
                            self.set(&decl.property, decl.value.clone());
                        }
                    }
                }
            }
        }
    }
}


pub fn is_inherited_property(property: &str) -> bool {
    
    if property.starts_with("--") {
        return true;
    }
    
    matches!(
        property,
        "color" |
        "font-family" |
        "font-size" |
        "font-style" |
        "font-weight" |
        "font-variant" |
        "font" |
        "letter-spacing" |
        "line-height" |
        "text-align" |
        "text-align-last" |
        "text-indent" |
        "text-justify" |
        "text-transform" |
        "text-decoration" |
        "text-decoration-color" |
        "text-decoration-line" |
        "text-decoration-style" |
        "text-shadow" |
        "text-overflow" |
        "text-rendering" |
        "white-space" |
        "word-break" |
        "word-wrap" |
        "overflow-wrap" |
        "hyphens" |
        "tab-size" |
        "visibility" |
        "cursor" |
        "quotes" |
        "orphans" |
        "widows" |
        "list-style" |
        "list-style-type" |
        "list-style-position" |
        "list-style-image" |
        "word-spacing"
    )
}





pub fn expand_shorthand(property: &str, value: &CssValue) -> Vec<(String, CssValue)> {
    match property {
        "margin" | "padding" => expand_box_model_shorthand(property, value),
        "border" | "border-top" | "border-right" | "border-bottom" | "border-left" => {
            expand_border_shorthand(property, value)
        }
        "border-width" | "border-style" | "border-color" => {
            expand_border_side_shorthand(property, value)
        }
        "background" => expand_background_shorthand(value),
        "font" => expand_font_shorthand(value),
        "list-style" => expand_list_style_shorthand(value),
        "flex" => expand_flex_shorthand(value),
        "grid-area" | "grid-column" | "grid-row" => expand_grid_shorthand(property, value),
        "outline" => expand_outline_shorthand(value),
        "transition" | "animation" | "text-decoration" | "column" | "gap" |
        "place-items" | "place-content" | "place-self" | "overflow" | "border-radius" => {
            
            
            vec![(property.to_string(), value.clone())]
        }
        _ => vec![(property.to_string(), value.clone())],
    }
}







fn expand_box_model_shorthand(property: &str, value: &CssValue) -> Vec<(String, CssValue)> {
    
    
    match value {
        CssValue::Keyword(v) => {
            let parts: Vec<&str> = v.split_whitespace().collect();
            let prefix = property; 
            match parts.len() {
                1 => {
                    let val = parse_box_value(parts[0]);
                    vec![
                        (format!("{}-top", prefix), val.clone()),
                        (format!("{}-right", prefix), val.clone()),
                        (format!("{}-bottom", prefix), val.clone()),
                        (format!("{}-left", prefix), val),
                    ]
                }
                2 => {
                    let val1 = parse_box_value(parts[0]);
                    let val2 = parse_box_value(parts[1]);
                    vec![
                        (format!("{}-top", prefix), val1.clone()),
                        (format!("{}-right", prefix), val2.clone()),
                        (format!("{}-bottom", prefix), val1.clone()),
                        (format!("{}-left", prefix), val2),
                    ]
                }
                3 => {
                    let val1 = parse_box_value(parts[0]);
                    let val2 = parse_box_value(parts[1]);
                    let val3 = parse_box_value(parts[2]);
                    vec![
                        (format!("{}-top", prefix), val1),
                        (format!("{}-right", prefix), val2.clone()),
                        (format!("{}-bottom", prefix), val3),
                        (format!("{}-left", prefix), val2),
                    ]
                }
                4 => {
                    let val1 = parse_box_value(parts[0]);
                    let val2 = parse_box_value(parts[1]);
                    let val3 = parse_box_value(parts[2]);
                    let val4 = parse_box_value(parts[3]);
                    vec![
                        (format!("{}-top", prefix), val1),
                        (format!("{}-right", prefix), val2),
                        (format!("{}-bottom", prefix), val3),
                        (format!("{}-left", prefix), val4),
                    ]
                }
                _ => vec![(property.to_string(), value.clone())],
            }
        }
        CssValue::Number(_) | CssValue::Length(_) => {
            let prefix = property;
            vec![
                (format!("{}-top", prefix), value.clone()),
                (format!("{}-right", prefix), value.clone()),
                (format!("{}-bottom", prefix), value.clone()),
                (format!("{}-left", prefix), value.clone()),
            ]
        }
        _ => vec![(property.to_string(), value.clone())],
    }
}

fn parse_box_value(s: &str) -> CssValue {
    if let Ok(n) = s.parse::<f32>() {
        CssValue::Number(n)
    } else if let Some(c) = Color::named(s) {
        CssValue::Color(c)
    } else if let Some(c) = Color::from_hex(s.strip_prefix('#').unwrap_or(s)) {
        CssValue::Color(c)
    } else {
        CssValue::Keyword(std::borrow::Cow::Owned(s.to_string()))
    }
}


fn expand_border_shorthand(property: &str, value: &CssValue) -> Vec<(String, CssValue)> {
    let prefix = match property {
        "border" => "",
        "border-top" => "-top",
        "border-right" => "-right",
        "border-bottom" => "-bottom",
        "border-left" => "-left",
        _ => "",
    };

    if let CssValue::Keyword(v) = value {
        let parts: Vec<&str> = v.split_whitespace().collect();
        let mut width = CssValue::Keyword(std::borrow::Cow::Borrowed("medium"));
        let mut style = CssValue::Keyword(std::borrow::Cow::Borrowed("none"));
        let mut color: Option<CssValue> = None;

        for part in &parts {
            let part_lower = part.to_lowercase();
            if part.chars().next().map(|c| c.is_digit(10) || c == '.').unwrap_or(false)
                || part_lower.ends_with("px") || part_lower.ends_with("em")
                || part_lower.ends_with("rem") || part_lower.ends_with("%")
                || matches!(part_lower.as_str(), "thin" | "medium" | "thick")
            {
                width = parse_box_value(part);
            } else if matches!(part_lower.as_str(),
                "none" | "hidden" | "dotted" | "dashed" | "solid" | "double" |
                "groove" | "ridge" | "inset" | "outset")
            {
                style = CssValue::Keyword(std::borrow::Cow::Owned(part.to_string()));
            } else if let Some(c) = parse_border_color(part) {
                color = Some(c);
            }
        }

        let mut result = vec![
            (format!("border{}-width", prefix), width),
            (format!("border{}-style", prefix), style),
        ];
        if let Some(c) = color {
            result.push((format!("border{}-color", prefix), c));
        }
        result
    } else {
        vec![(property.to_string(), value.clone())]
    }
}

fn parse_border_color(s: &str) -> Option<CssValue> {
    if s.starts_with('#') {
        Color::from_hex(&s[1..]).map(CssValue::Color)
    } else if s.starts_with("rgb(") {
        parse_rgb_color(s)
    } else if s.starts_with("rgba(") {
        parse_rgba_color(s)
    } else {
        Color::named(s).map(CssValue::Color)
    }
}

fn parse_rgb_color(s: &str) -> Option<CssValue> {
    let nums: Vec<&str> = s.trim_start_matches("rgb(").trim_end_matches(')').split(',').collect();
    if nums.len() == 3 {
        let r = nums[0].trim().parse::<u8>().ok()?;
        let g = nums[1].trim().parse::<u8>().ok()?;
        let b = nums[2].trim().parse::<u8>().ok()?;
        Some(CssValue::Color(Color::from_rgb(r, g, b)))
    } else {
        None
    }
}

fn parse_rgba_color(s: &str) -> Option<CssValue> {
    let parts: Vec<&str> = s.trim_start_matches("rgba(").trim_end_matches(')').split(',').collect();
    if parts.len() == 4 {
        let r = parts[0].trim().parse::<u8>().ok()?;
        let g = parts[1].trim().parse::<u8>().ok()?;
        let b = parts[2].trim().parse::<u8>().ok()?;
        let a = parts[3].trim().parse::<f32>().ok()?;
        Some(CssValue::Color(Color::from_rgba(r, g, b, (a * 255.0) as u8)))
    } else {
        None
    }
}


fn expand_border_side_shorthand(property: &str, value: &CssValue) -> Vec<(String, CssValue)> {
    let base = property.trim_end_matches("-width").trim_end_matches("-style").trim_end_matches("-color");
    let suffix = if property.ends_with("-width") { "-width" }
        else if property.ends_with("-style") { "-style" }
        else { "-color" };

    if let CssValue::Keyword(v) = value {
        let parts: Vec<&str> = v.split_whitespace().collect();
        match parts.len() {
            1 => {
                let val = parse_box_value(parts[0]);
                vec![
                    (format!("{}-top{}", base, suffix), val.clone()),
                    (format!("{}-right{}", base, suffix), val.clone()),
                    (format!("{}-bottom{}", base, suffix), val.clone()),
                    (format!("{}-left{}", base, suffix), val),
                ]
            }
            2 => {
                let val1 = parse_box_value(parts[0]);
                let val2 = parse_box_value(parts[1]);
                vec![
                    (format!("{}-top{}", base, suffix), val1.clone()),
                    (format!("{}-right{}", base, suffix), val2.clone()),
                    (format!("{}-bottom{}", base, suffix), val1.clone()),
                    (format!("{}-left{}", base, suffix), val2),
                ]
            }
            4 => {
                let vals: Vec<_> = parts.iter().map(|p| parse_box_value(p)).collect();
                vec![
                    (format!("{}-top{}", base, suffix), vals[0].clone()),
                    (format!("{}-right{}", base, suffix), vals[1].clone()),
                    (format!("{}-bottom{}", base, suffix), vals[2].clone()),
                    (format!("{}-left{}", base, suffix), vals[3].clone()),
                ]
            }
            _ => vec![(property.to_string(), value.clone())],
        }
    } else {
        vec![(property.to_string(), value.clone())]
    }
}



fn expand_background_shorthand(value: &CssValue) -> Vec<(String, CssValue)> {
    let mut color: Option<CssValue> = None;
    let mut image: Option<CssValue> = None;
    let mut position: Option<CssValue> = None;
    let mut size: Option<CssValue> = None;
    let mut repeat: Option<CssValue> = None;
    let mut attachment: Option<CssValue> = None;
    let mut origin: Option<CssValue> = None;
    let mut clip: Option<CssValue> = None;

    let css_str = match value {
        CssValue::Keyword(s) => s.to_string(),
        CssValue::String(s) => s.to_string(),
        _ => return vec![("background".to_string(), value.clone())],
    };

    let parts: Vec<&str> = css_str.split_whitespace().collect();
    let mut i = 0;

    while i < parts.len() {
        let part = parts[i].to_lowercase();

        
        if part.starts_with('#') || Color::named(&part).is_some() {
            if color.is_none() {
                if let Some(c) = Color::named(&part) {
                    color = Some(CssValue::Color(c));
                } else if let Some(c) = Color::from_hex(&part[1..]) {
                    color = Some(CssValue::Color(c));
                }
            }
        } else if part == "none" {
            if image.is_none() {
                image = Some(CssValue::Keyword(std::borrow::Cow::Borrowed("none")));
            }
        } else if part.starts_with("url(") {
            
            let mut url_str = part.to_string();
            while !url_str.ends_with(')') && i + 1 < parts.len() {
                i += 1;
                url_str.push(' ');
                url_str.push_str(parts[i]);
            }
            if image.is_none() {
                image = Some(CssValue::Keyword(std::borrow::Cow::Owned(url_str)));
            }
        } else if matches!(part.as_str(), "repeat" | "no-repeat" | "repeat-x" | "repeat-y" | "space" | "round") {
            if repeat.is_none() {
                repeat = Some(CssValue::Keyword(std::borrow::Cow::Owned(part.clone())));
            }
        } else if matches!(part.as_str(), "scroll" | "fixed" | "local") {
            if attachment.is_none() {
                attachment = Some(CssValue::Keyword(std::borrow::Cow::Owned(part.clone())));
            }
        } else if matches!(part.as_str(), "padding-box" | "border-box" | "content-box") {
            if origin.is_none() {
                origin = Some(CssValue::Keyword(std::borrow::Cow::Owned(part.clone())));
            } else if clip.is_none() {
                clip = Some(CssValue::Keyword(std::borrow::Cow::Owned(part.clone())));
            }
        } else if part == "/" {
            
            if i + 1 < parts.len() {
                i += 1;
                let mut size_str = parts[i].to_string();
                
                while i + 1 < parts.len() && !parts[i + 1].starts_with("url(")
                    && !matches!(parts[i + 1], "repeat" | "no-repeat" | "scroll" | "fixed" | "local"
                        | "padding-box" | "border-box" | "content-box")
                    && !parts[i + 1].starts_with('#') {
                    i += 1;
                    size_str.push(' ');
                    size_str.push_str(parts[i]);
                }
                size = Some(CssValue::Keyword(std::borrow::Cow::Owned(size_str)));
            }
        } else if part.starts_with("linear-gradient(") || part.starts_with("radial-gradient(") {
            
            let mut grad_str = part.to_string();
            while !grad_str.ends_with(')') && i + 1 < parts.len() {
                i += 1;
                grad_str.push(' ');
                grad_str.push_str(parts[i]);
            }
            if image.is_none() {
                image = Some(CssValue::Keyword(std::borrow::Cow::Owned(grad_str)));
            }
        } else {
            
            if position.is_none() {
                let mut pos_str = part.to_string();
                
                if i + 1 < parts.len() {
                    let next = parts[i + 1].to_lowercase();
                    if next.starts_with(|c: char| c.is_digit(10) || c == '-')
                        || matches!(next.as_str(), "top" | "bottom" | "left" | "right" | "center") {
                        i += 1;
                        pos_str.push(' ');
                        pos_str.push_str(parts[i]);
                    }
                }
                position = Some(CssValue::Keyword(std::borrow::Cow::Owned(pos_str)));
            }
        }

        i += 1;
    }

    let mut result = Vec::new();
    result.push(("background-color".to_string(), color.unwrap_or(CssValue::Keyword(std::borrow::Cow::Borrowed("transparent")))));
    result.push(("background-image".to_string(), image.unwrap_or(CssValue::Keyword(std::borrow::Cow::Borrowed("none")))));
    result.push(("background-position".to_string(), position.unwrap_or(CssValue::Keyword(std::borrow::Cow::Borrowed("0% 0%")))));
    if let Some(s) = size {
        result.push(("background-size".to_string(), s));
    }
    result.push(("background-repeat".to_string(), repeat.unwrap_or(CssValue::Keyword(std::borrow::Cow::Borrowed("repeat")))));
    result.push(("background-attachment".to_string(), attachment.unwrap_or(CssValue::Keyword(std::borrow::Cow::Borrowed("scroll")))));
    result.push(("background-origin".to_string(), origin.unwrap_or(CssValue::Keyword(std::borrow::Cow::Borrowed("padding-box")))));
    result.push(("background-clip".to_string(), clip.unwrap_or(CssValue::Keyword(std::borrow::Cow::Borrowed("border-box")))));

    result
}



fn expand_font_shorthand(value: &CssValue) -> Vec<(String, CssValue)> {
    let css_str = match value {
        CssValue::Keyword(s) => s.to_string(),
        CssValue::String(s) => s.to_string(),
        _ => return vec![("font".to_string(), value.clone())],
    };

    let mut style = "normal".to_string();
    let mut variant = "normal".to_string();
    let mut weight = "normal".to_string();
    let mut size: Option<String> = None;
    let mut line_height: Option<String> = None;
    let mut family_parts = Vec::new();
    let mut parsing_family = false;

    let parts: Vec<&str> = css_str.split_whitespace().collect();
    let mut i = 0;

    while i < parts.len() {
        let part = parts[i].to_lowercase();

        if parsing_family {
            family_parts.push(parts[i].to_string());
        } else if matches!(part.as_str(), "normal") {
            
            
            if style == "normal" {
                
            }
        } else if matches!(part.as_str(), "italic" | "oblique") {
            style = part;
        } else if part == "small-caps" {
            variant = part;
        } else if matches!(part.as_str(), "bold" | "bolder" | "lighter" | "100" | "200" | "300" | "400" | "500" | "600" | "700" | "800" | "900") {
            weight = part;
        } else if part.contains('/') || part.chars().next().map(|c| c.is_digit(10)).unwrap_or(false)
            || part.ends_with("px") || part.ends_with("em") || part.ends_with("rem")
            || part.ends_with("pt") || part.ends_with("%") || part == "medium" || part == "large"
            || part == "small" || part == "x-large" || part == "x-small" || part == "larger" || part == "smaller" {
            
            if part.contains('/') {
                let size_parts: Vec<&str> = part.split('/').collect();
                size = Some(size_parts[0].to_string());
                if size_parts.len() > 1 {
                    line_height = Some(size_parts[1].to_string());
                }
            } else {
                size = Some(part.clone());
                
                if i + 1 < parts.len() && parts[i + 1].starts_with('/') {
                    if i + 2 < parts.len() {
                        line_height = Some(parts[i + 2].trim_start_matches('/').to_string());
                        i += 1; 
                    }
                }
            }
            parsing_family = true;
        } else {
            
            parsing_family = true;
            family_parts.push(parts[i].to_string());
        }

        i += 1;
    }

    let font_family = if family_parts.is_empty() {
        "sans-serif".to_string()
    } else {
        family_parts.join(" ")
    };

    vec![
        ("font-style".to_string(), CssValue::Keyword(std::borrow::Cow::Owned(style))),
        ("font-variant".to_string(), CssValue::Keyword(std::borrow::Cow::Owned(variant))),
        ("font-weight".to_string(), CssValue::Keyword(std::borrow::Cow::Owned(weight))),
        ("font-size".to_string(), CssValue::Keyword(std::borrow::Cow::Owned(size.unwrap_or("16px".to_string())))),
        ("line-height".to_string(), CssValue::Keyword(std::borrow::Cow::Owned(line_height.unwrap_or("normal".to_string())))),
        ("font-family".to_string(), CssValue::Keyword(std::borrow::Cow::Owned(font_family))),
    ]
}


fn expand_list_style_shorthand(_value: &CssValue) -> Vec<(String, CssValue)> {
    vec![("list-style".to_string(), _value.clone())]
}


fn expand_flex_shorthand(_value: &CssValue) -> Vec<(String, CssValue)> {
    vec![("flex".to_string(), _value.clone())]
}


fn expand_grid_shorthand(_property: &str, _value: &CssValue) -> Vec<(String, CssValue)> {
    vec![("grid".to_string(), _value.clone())]
}


fn expand_outline_shorthand(_value: &CssValue) -> Vec<(String, CssValue)> {
    vec![("outline".to_string(), _value.clone())]
}








pub fn inherit_properties(
    node_styles: &mut HashMap<String, CssValue>,
    parent_styles: &HashMap<String, CssValue>,
) {
    for (property, value) in parent_styles {
        
        if node_styles.contains_key(property) {
            
            if let Some(CssValue::Keyword(k)) = node_styles.get(property) {
                if k.as_ref() != "inherit" {
                    continue;
                }
            } else {
                continue;
            }
        }

        
        if matches!(value, CssValue::Initial | CssValue::Unset) {
            continue;
        }

        
        if is_inherited_property(property) {
            node_styles.insert(property.clone(), value.clone());
        }
    }
}



pub fn parse_border_shorthand(value: &str) -> Option<(String, String, Option<CssValue>)> {
    let parts: Vec<&str> = value.split_whitespace().collect();

    let mut width = String::from("medium");
    let mut style = String::from("none");
    let mut color: Option<CssValue> = None;

    for part in &parts {
        let part_lower = part.to_lowercase();

        if part.chars().next().map(|c| c.is_digit(10) || c == '.').unwrap_or(false)
            || part_lower.ends_with("px") || part_lower.ends_with("em")
            || part_lower.ends_with("rem") || part_lower.ends_with("%")
            || matches!(part_lower.as_str(), "thin" | "medium" | "thick")
        {
            width = part.to_string();
        } else if matches!(part_lower.as_str(),
            "none" | "hidden" | "dotted" | "dashed" | "solid" | "double" |
            "groove" | "ridge" | "inset" | "outset")
        {
            style = part.to_string();
        } else if let Some(css_color) = parse_border_color(part) {
            color = Some(css_color);
        }
    }

    Some((width, style, color))
}


pub fn parse_background_image(value: &str) -> Option<String> {
    let value = value.trim();

    if value.eq_ignore_ascii_case("none") {
        return None;
    }

    if value.starts_with("url(") && value.ends_with(')') {
        let url = &value[4..value.len()-1].trim().trim_matches('"').trim_matches('\'');
        return Some(url.to_string());
    }

    Some(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_properties() {
        let mut props = CustomProperties::new();
        props.set("--primary-color", CssValue::Keyword(std::borrow::Cow::Borrowed("blue")));
        props.set("--font-size", CssValue::Number(16.0));

        assert!(props.get("--primary-color").is_some());
        assert!(props.get("--nonexistent").is_none());

        let resolved = props.resolve_var("var(--primary-color)");
        assert_eq!(resolved, "blue");

        let resolved_with_fallback = props.resolve_var("var(--missing, red)");
        assert_eq!(resolved_with_fallback, "red");
    }

    #[test]
    fn test_expand_margin_one_value() {
        let value = CssValue::Number(10.0);
        let expanded = expand_box_model_shorthand("margin", &value);
        assert_eq!(expanded.len(), 4);
        assert_eq!(expanded[0].0, "margin-top");
        assert_eq!(expanded[1].0, "margin-right");
        assert_eq!(expanded[2].0, "margin-bottom");
        assert_eq!(expanded[3].0, "margin-left");
    }

    #[test]
    fn test_expand_margin_two_values() {
        let value = CssValue::Keyword(std::borrow::Cow::Owned("10px 20px".to_string()));
        let expanded = expand_box_model_shorthand("margin", &value);
        assert_eq!(expanded.len(), 4);
        
    }

    #[test]
    fn test_inherit_only_inheritable() {
        let mut parent = HashMap::new();
        parent.insert("color".to_string(), CssValue::Keyword(std::borrow::Cow::Borrowed("red")));
        parent.insert("width".to_string(), CssValue::Number(100.0));

        let mut child = HashMap::new();
        inherit_properties(&mut child, &parent);

        
        assert!(child.contains_key("color"));
        
        assert!(!child.contains_key("width"));
    }
}
