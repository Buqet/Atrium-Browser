use crate::css::value::{CssLength, CssCalcExpression, ViewportContext};


#[derive(Debug)]
pub enum ExprToken {
    Value(String),
    Operator(char),
    ParenOpen,
    ParenClose,
    Function(String), 
}





pub fn evaluate_calc(expression: &str, context: Option<&ViewportContext>) -> Option<String> {
    let expr = expression.trim();

    if !expr.starts_with("calc(") || !expr.ends_with(')') {
        return None;
    }

    let inner = &expr[5..expr.len() - 1].trim();

    let default_ctx = ViewportContext::new(1920.0, 1080.0);
    let ctx = context.unwrap_or(&default_ctx);

    
    if let Some(calc_expr) = parse_css_calc_expression(inner, ctx) {
        let result = calc_expr.evaluate(ctx);
        if result.fract() == 0.0 {
            Some(format!("{}px", result as i32))
        } else {
            Some(format!("{:.2}px", result))
        }
    } else {
        
        let tokens = tokenize_expression(inner);
        if tokens.is_empty() {
            return None;
        }

        let mut pos = 0;
        let result = parse_additive(&tokens, &mut pos, ctx)?;

        if result.fract() == 0.0 {
            Some(format!("{}px", result as i32))
        } else {
            Some(format!("{:.2}px", result))
        }
    }
}


fn parse_css_calc_expression(expr: &str, ctx: &ViewportContext) -> Option<CssCalcExpression> {
    let expr = expr.trim();
    
    
    if let Some((func_name, ref args_str)) = extract_function(expr) {
        match func_name.to_lowercase().as_str() {
            "abs" => {
                let inner = parse_css_calc_expression(args_str, ctx)?;
                Some(CssCalcExpression::Abs(Box::new(inner)))
            }
            "sign" => {
                let inner = parse_css_calc_expression(args_str, ctx)?;
                Some(CssCalcExpression::Sign(Box::new(inner)))
            }
            "sqrt" => {
                let inner = parse_css_calc_expression(args_str, ctx)?;
                Some(CssCalcExpression::Sqrt(Box::new(inner)))
            }
            "exp" => {
                let inner = parse_css_calc_expression(args_str, ctx)?;
                Some(CssCalcExpression::Exp(Box::new(inner)))
            }
            "log" => {
                let inner = parse_css_calc_expression(args_str, ctx)?;
                Some(CssCalcExpression::Log(Box::new(inner)))
            }
            "sin" => {
                let inner = parse_css_calc_expression(args_str, ctx)?;
                Some(CssCalcExpression::Sin(Box::new(inner)))
            }
            "cos" => {
                let inner = parse_css_calc_expression(args_str, ctx)?;
                Some(CssCalcExpression::Cos(Box::new(inner)))
            }
            "tan" => {
                let inner = parse_css_calc_expression(args_str, ctx)?;
                Some(CssCalcExpression::Tan(Box::new(inner)))
            }
            "asin" => {
                let inner = parse_css_calc_expression(args_str, ctx)?;
                Some(CssCalcExpression::Asin(Box::new(inner)))
            }
            "acos" => {
                let inner = parse_css_calc_expression(args_str, ctx)?;
                Some(CssCalcExpression::Acos(Box::new(inner)))
            }
            "atan" => {
                let inner = parse_css_calc_expression(args_str, ctx)?;
                Some(CssCalcExpression::Atan(Box::new(inner)))
            }
            "min" => {
                let args = split_function_args(args_str);
                let exprs: Vec<_> = args.iter()
                    .filter_map(|a| parse_css_calc_expression(a.trim(), ctx))
                    .collect();
                if exprs.is_empty() { return None; }
                Some(CssCalcExpression::Min(exprs))
            }
            "max" => {
                let args = split_function_args(args_str);
                let exprs: Vec<_> = args.iter()
                    .filter_map(|a| parse_css_calc_expression(a.trim(), ctx))
                    .collect();
                if exprs.is_empty() { return None; }
                Some(CssCalcExpression::Max(exprs))
            }
            "clamp" => {
                let args: Vec<_> = split_function_args(args_str);
                if args.len() != 3 { return None; }
                let min = parse_css_calc_expression(args[0].trim(), ctx)?;
                let val = parse_css_calc_expression(args[1].trim(), ctx)?;
                let max = parse_css_calc_expression(args[2].trim(), ctx)?;
                Some(CssCalcExpression::Clamp {
                    min: Box::new(min),
                    val: Box::new(val),
                    max: Box::new(max),
                })
            }
            _ => None,
        }
    } else {
        
        parse_arithmetic_expression(expr, ctx)
    }
}


fn extract_function(expr: &str) -> Option<(String, String)> {
    let paren_idx = expr.find('(')?;
    if !expr.ends_with(')') {
        return None;
    }
    let name = expr[..paren_idx].trim().to_string();
    let args = expr[paren_idx + 1..expr.len() - 1].trim().to_string();
    Some((name, args))
}


fn parse_arithmetic_expression(expr: &str, ctx: &ViewportContext) -> Option<CssCalcExpression> {
    
    let mut depth = 0;
    let chars: Vec<char> = expr.chars().collect();
    
    
    for i in (0..chars.len()).rev() {
        let c = chars[i];
        if c == '(' { depth += 1; }
        else if c == ')' { depth -= 1; }
        else if depth == 0 && (c == '+' || c == '-') && i > 0 {
            let left = expr[..i].trim();
            let right = expr[i+1..].trim();
            let left_expr = parse_css_calc_expression(left, ctx)?;
            let right_expr = parse_css_calc_expression(right, ctx)?;
            if c == '+' {
                return Some(CssCalcExpression::Add(Box::new(left_expr), Box::new(right_expr)));
            } else {
                return Some(CssCalcExpression::Sub(Box::new(left_expr), Box::new(right_expr)));
            }
        }
    }
    
    
    for i in (0..chars.len()).rev() {
        let c = chars[i];
        if c == '(' { depth += 1; }
        else if c == ')' { depth -= 1; }
        else if depth == 0 && (c == '*' || c == '/') && i > 0 && i < chars.len() - 1 {
            let left = expr[..i].trim();
            let right = expr[i+1..].trim();
            let left_expr = parse_css_calc_expression(left, ctx)?;
            let right_expr = parse_css_calc_expression(right, ctx)?;
            if c == '*' {
                return Some(CssCalcExpression::Mul(Box::new(left_expr), Box::new(right_expr)));
            } else {
                return Some(CssCalcExpression::Div(Box::new(left_expr), Box::new(right_expr)));
            }
        }
    }
    
    
    if expr.starts_with('(') && expr.ends_with(')') {
        let inner = &expr[1..expr.len()-1];
        
        let mut d = 0;
        let mut matching = true;
        for (i, c) in inner.chars().enumerate() {
            if c == '(' { d += 1; }
            else if c == ')' {
                if d == 0 { matching = false; break; }
                d -= 1;
            }
        }
        if matching && d == 0 {
            return parse_css_calc_expression(inner, ctx);
        }
    }
    
    
    parse_length_to_calc(expr, ctx)
}


fn parse_length_to_calc(expr: &str, ctx: &ViewportContext) -> Option<CssCalcExpression> {
    let len = parse_length_value(expr)?;
    Some(CssCalcExpression::Length(len))
}


fn parse_length_value(value: &str) -> Option<CssLength> {
    let value = value.trim();
    if value.is_empty() { return None; }
    
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


pub fn evaluate_css_function(expression: &str, context: Option<&ViewportContext>) -> Option<String> {
    let expr = expression.trim();

    let paren_start = expr.find('(')?;
    let func_name = &expr[..paren_start].to_lowercase();

    if !expr.ends_with(')') {
        return None;
    }

    let inner = &expr[paren_start + 1..expr.len() - 1];
    let args = split_function_args(inner);

    let default_ctx = ViewportContext::new(1920.0, 1080.0);
    let ctx = context.unwrap_or(&default_ctx);

    match func_name.as_str() {
        "min" => evaluate_min_max(&args, true, ctx),
        "max" => evaluate_min_max(&args, false, ctx),
        "clamp" => {
            if args.len() != 3 {
                return None;
            }
            evaluate_clamp(&args, ctx)
        }
        _ => None,
    }
}


fn tokenize_expression(expr: &str) -> Vec<ExprToken> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c.is_whitespace() {
            if !current.is_empty() {
                tokens.push(ExprToken::Value(current.clone()));
                current.clear();
            }
            i += 1;
            continue;
        }

        if c == '(' {
            if !current.is_empty() {
                tokens.push(ExprToken::Value(current.clone()));
                current.clear();
            }
            
            let mut depth = 1;
            let start = i + 1;
            i += 1;
            while i < chars.len() && depth > 0 {
                if chars[i] == '(' {
                    depth += 1;
                } else if chars[i] == ')' {
                    depth -= 1;
                }
                i += 1;
            }
            let inner = &expr[start..i - 1];
            tokens.push(ExprToken::ParenOpen);
            
            let inner_tokens = tokenize_expression(inner);
            tokens.extend(inner_tokens);
            tokens.push(ExprToken::ParenClose);
            continue;
        }

        if matches!(c, '+' | '-' | '*' | '/') {
            if !current.is_empty() {
                
                if c == '-' && tokens.last().map_or(true, |t| matches!(t, ExprToken::Operator(_))) {
                    current.push(c);
                    i += 1;
                    continue;
                }
                tokens.push(ExprToken::Value(current.clone()));
                current.clear();
            }
            tokens.push(ExprToken::Operator(c));
            i += 1;
            continue;
        }

        current.push(c);
        i += 1;
    }

    if !current.is_empty() {
        tokens.push(ExprToken::Value(current));
    }

    tokens
}


fn str_to_px(value: &str, context: &ViewportContext) -> Option<f32> {
    let value = value.trim();

    let mut num_str = String::new();
    let mut unit = String::new();

    for c in value.chars() {
        if c.is_digit(10) || c == '.' || c == '-' {
            num_str.push(c);
        } else if !c.is_whitespace() {
            unit.push(c);
        }
    }

    let num: f32 = num_str.parse().ok()?;

    match unit.as_str() {
        "px" => Some(num),
        "%" => Some(num), 
        "em" | "rem" => Some(num * 16.0),
        "vw" => Some(num * context.viewport_width / 100.0),
        "vh" => Some(num * context.viewport_height / 100.0),
        "" => Some(num),
        _ => Some(num),
    }
}




fn parse_additive(tokens: &[ExprToken], pos: &mut usize, ctx: &ViewportContext) -> Option<f32> {
    let mut left = parse_multiplicative(tokens, pos, ctx)?;

    while *pos < tokens.len() {
        if let ExprToken::Operator(op) = &tokens[*pos] {
            if *op == '+' || *op == '-' {
                let op = *op;
                *pos += 1;
                let right = parse_multiplicative(tokens, pos, ctx)?;
                match op {
                    '+' => left += right,
                    '-' => left -= right,
                    _ => unreachable!(),
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    Some(left)
}


fn parse_multiplicative(tokens: &[ExprToken], pos: &mut usize, ctx: &ViewportContext) -> Option<f32> {
    let mut left = parse_primary(tokens, pos, ctx)?;

    while *pos < tokens.len() {
        if let ExprToken::Operator(op) = &tokens[*pos] {
            if *op == '*' || *op == '/' {
                let op = *op;
                *pos += 1;
                let right = parse_primary(tokens, pos, ctx)?;
                match op {
                    '*' => left *= right,
                    '/' => {
                        if right != 0.0 {
                            left /= right;
                        } else {
                            return None;
                        }
                    }
                    _ => unreachable!(),
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    Some(left)
}


fn parse_primary(tokens: &[ExprToken], pos: &mut usize, ctx: &ViewportContext) -> Option<f32> {
    if *pos >= tokens.len() {
        return None;
    }

    match &tokens[*pos] {
        ExprToken::ParenOpen => {
            *pos += 1; 
            let result = parse_additive(tokens, pos, ctx)?;
            if *pos < tokens.len() && matches!(tokens[*pos], ExprToken::ParenClose) {
                *pos += 1; 
            }
            Some(result)
        }
        ExprToken::Value(v) => {
            *pos += 1;
            str_to_px(v, ctx)
        }
        _ => None,
    }
}




fn split_function_args(inner: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for c in inner.chars() {
        if c == ',' && depth == 0 {
            args.push(current.trim().to_string());
            current.clear();
        } else {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
            }
            current.push(c);
        }
    }

    if !current.trim().is_empty() {
        args.push(current.trim().to_string());
    }

    args
}


fn evaluate_min_max(args: &[String], find_min: bool, ctx: &ViewportContext) -> Option<String> {
    if args.is_empty() {
        return None;
    }

    let mut values = Vec::new();

    for arg in args {
        let px = str_to_px(arg, ctx)?;
        values.push(px);
    }

    let result = if find_min {
        *values.iter().min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))?
    } else {
        *values.iter().max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))?
    };

    if result.fract() == 0.0 {
        Some(format!("{}px", result as i32))
    } else {
        Some(format!("{:.2}px", result))
    }
}


fn evaluate_clamp(args: &[String], ctx: &ViewportContext) -> Option<String> {
    let min_val = str_to_px(&args[0], ctx)?;
    let value = str_to_px(&args[1], ctx)?;
    let max_val = str_to_px(&args[2], ctx)?;

    let clamped = value.max(min_val).min(max_val);

    if clamped.fract() == 0.0 {
        Some(format!("{}px", clamped as i32))
    } else {
        Some(format!("{:.2}px", clamped))
    }
}


pub fn parse_transform(value: &str) -> Vec<crate::css::value::CssTransform> {
    use crate::css::value::CssTransform;

    let mut transforms = Vec::new();
    let value = value.trim();

    if value.is_empty() || value.eq_ignore_ascii_case("none") {
        return transforms;
    }

    let mut remaining = value;

    while let Some(paren_pos) = remaining.find('(') {
        let func_name = remaining[..paren_pos].trim().to_lowercase();

        if let Some(end_paren) = remaining[paren_pos..].find(')') {
            let end_paren = paren_pos + end_paren;
            let inner = &remaining[paren_pos + 1..end_paren];

            let args: Vec<f32> = inner.split(',')
                .filter_map(|s| {
                    let s = s.trim();
                    let mut num_str = String::new();
                    for c in s.chars() {
                        if c.is_digit(10) || c == '.' || c == '-' {
                            num_str.push(c);
                        } else {
                            break;
                        }
                    }
                    num_str.parse().ok()
                })
                .collect();

            match func_name.as_str() {
                "translatex" => {
                    if let Some(x) = args.first() {
                        transforms.push(CssTransform::TranslateX(*x));
                    }
                }
                "translatey" => {
                    if let Some(y) = args.first() {
                        transforms.push(CssTransform::TranslateY(*y));
                    }
                }
                "translate" => {
                    if args.len() >= 2 {
                        transforms.push(CssTransform::Translate(args[0], args[1]));
                    } else if let Some(x) = args.first() {
                        transforms.push(CssTransform::Translate(*x, 0.0));
                    }
                }
                "rotate" => {
                    if let Some(deg) = args.first() {
                        transforms.push(CssTransform::Rotate(*deg));
                    }
                }
                "scale" => {
                    if let Some(s) = args.first() {
                        transforms.push(CssTransform::Scale(*s));
                    }
                }
                "scalex" => {
                    if let Some(s) = args.first() {
                        transforms.push(CssTransform::ScaleX(*s));
                    }
                }
                "scaley" => {
                    if let Some(s) = args.first() {
                        transforms.push(CssTransform::ScaleY(*s));
                    }
                }
                "skewx" => {
                    if let Some(deg) = args.first() {
                        transforms.push(CssTransform::SkewX(*deg));
                    }
                }
                "skewy" => {
                    if let Some(deg) = args.first() {
                        transforms.push(CssTransform::SkewY(*deg));
                    }
                }
                "matrix" => {
                    if args.len() == 6 {
                        transforms.push(CssTransform::Matrix(
                            args[0], args[1], args[2], args[3], args[4], args[5]
                        ));
                    }
                }
                _ => {}
            }

            remaining = &remaining[end_paren + 1..];
        } else {
            break;
        }
    }

    transforms
}


pub fn parse_box_shadow(value: &str) -> Vec<crate::css::value::CssBoxShadow> {
    use crate::css::value::{CssBoxShadow, Color};

    let mut shadows = Vec::new();

    for shadow_str in value.split(',') {
        let shadow_str = shadow_str.trim();
        let parts: Vec<&str> = shadow_str.split_whitespace().collect();

        if parts.is_empty() {
            continue;
        }

        let mut offset_x = 0.0;
        let mut offset_y = 0.0;
        let mut blur_radius = 0.0;
        let mut spread_radius = 0.0;
        let mut color: Option<Color> = None;
        let mut inset = false;

        let mut value_idx = 0;

        for part in &parts {
            let part_lower = part.to_lowercase();

            if part_lower == "inset" {
                inset = true;
                continue;
            }

            
            if let Some(c) = parse_border_color(part) {
                if let crate::css::value::CssValue::Color(col) = c {
                    color = Some(col);
                }
                continue;
            }

            if let Some(px) = str_to_px(part, &ViewportContext::new(1920.0, 1080.0)) {
                match value_idx {
                    0 => offset_x = px,
                    1 => offset_y = px,
                    2 => blur_radius = px,
                    3 => spread_radius = px,
                    _ => {}
                }
                value_idx += 1;
            }
        }

        if value_idx >= 2 {
            shadows.push(CssBoxShadow {
                offset_x,
                offset_y,
                blur_radius,
                spread_radius,
                color,
                inset,
            });
        }
    }

    shadows
}


pub fn parse_css_filter(value: &str) -> Vec<crate::css::value::CssFilter> {
    use crate::css::value::CssFilter;

    let mut filters = Vec::new();
    let value = value.trim();

    if value.is_empty() || value.eq_ignore_ascii_case("none") {
        return filters;
    }

    let mut remaining = value;

    while let Some(paren_pos) = remaining.find('(') {
        let func_name = remaining[..paren_pos].trim().to_lowercase();

        if let Some(end_paren) = remaining[paren_pos..].find(')') {
            let end_paren = paren_pos + end_paren;
            let inner = remaining[paren_pos + 1..end_paren].trim();

            let filter = if inner.eq_ignore_ascii_case("none") {
                None
            } else {
                let mut num_str = String::new();
                for c in inner.chars() {
                    if c.is_digit(10) || c == '.' || c == '-' {
                        num_str.push(c);
                    } else if !c.is_whitespace() {
                        break;
                    }
                }

                if let Ok(num) = num_str.parse::<f32>() {
                    match func_name.as_str() {
                        "blur" => Some(CssFilter::Blur(num)),
                        "brightness" => Some(CssFilter::Brightness(num)),
                        "contrast" => Some(CssFilter::Contrast(num)),
                        "grayscale" => Some(CssFilter::Grayscale(num)),
                        "sepia" => Some(CssFilter::Sepia(num)),
                        "saturate" => Some(CssFilter::Saturate(num)),
                        "hue-rotate" => Some(CssFilter::HueRotate(num)),
                        "invert" => Some(CssFilter::Invert(num)),
                        "opacity" => Some(CssFilter::Opacity(num)),
                        _ => None,
                    }
                } else {
                    None
                }
            };

            if let Some(f) = filter {
                filters.push(f);
            }

            remaining = &remaining[end_paren + 1..];
        } else {
            break;
        }
    }

    filters
}


pub fn parse_transition(value: &str) -> Vec<crate::css::parser::CssTransition> {
    use crate::css::parser::CssTransition;

    let mut transitions = Vec::new();

    for trans_str in value.split(',') {
        let trans_str = trans_str.trim();
        let parts: Vec<&str> = trans_str.split_whitespace().collect();

        if parts.is_empty() {
            continue;
        }

        let mut property = String::from("all");
        let mut duration = 0.0;
        let mut timing_function = String::from("ease");
        let mut delay = 0.0;

        let mut value_idx = 0;

        for part in &parts {
            let part_lower = part.to_lowercase();

            if part_lower.ends_with("ms") {
                let num = part[..part.len() - 2].parse::<f32>().ok();
                if let Some(ms) = num {
                    if value_idx == 1 {
                        duration = ms / 1000.0;
                    } else {
                        delay = ms / 1000.0;
                    }
                    value_idx += 1;
                }
            } else if part_lower.ends_with('s') {
                let num = part[..part.len() - 1].parse::<f32>().ok();
                if let Some(sec) = num {
                    if value_idx == 1 {
                        duration = sec;
                    } else {
                        delay = sec;
                    }
                    value_idx += 1;
                }
            } else if matches!(part_lower.as_str(),
                "ease" | "ease-in" | "ease-out" | "ease-in-out" | "linear" | "step-start" | "step-end")
            {
                timing_function = part.to_string();
            } else {
                property = part.to_string();
            }
        }

        transitions.push(CssTransition {
            property,
            duration,
            timing_function,
            delay,
        });
    }

    transitions
}


fn parse_border_color(s: &str) -> Option<crate::css::value::CssValue> {
    use crate::css::value::{CssValue, Color};

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

fn parse_rgb_color(s: &str) -> Option<crate::css::value::CssValue> {
    use crate::css::value::{CssValue, Color};

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

fn parse_rgba_color(s: &str) -> Option<crate::css::value::CssValue> {
    use crate::css::value::{CssValue, Color};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_basic() {
        let result = evaluate_calc("calc(100px + 50px)", None);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "150px");
    }

    #[test]
    fn test_calc_with_parens() {
        let result = evaluate_calc("calc(2 * (100px - 20px))", None);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "160px");
    }

    #[test]
    fn test_calc_precedence() {
        
        let result = evaluate_calc("calc(100px + 50px * 2)", None);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "200px");
    }

    #[test]
    fn test_min_max_clamp() {
        let min_result = evaluate_css_function("min(100px, 200px)", None);
        assert!(min_result.is_some());
        assert_eq!(min_result.unwrap(), "100px");

        let max_result = evaluate_css_function("max(100px, 200px)", None);
        assert!(max_result.is_some());
        assert_eq!(max_result.unwrap(), "200px");

        let clamp_result = evaluate_css_function("clamp(50px, 150px, 200px)", None);
        assert!(clamp_result.is_some());
        assert_eq!(clamp_result.unwrap(), "150px");
    }

    #[test]
    fn test_transform_parse() {
        let transforms = parse_transform("translateX(10px) rotate(45deg) scale(2)");
        assert_eq!(transforms.len(), 3);
    }

    #[test]
    fn test_box_shadow_parse() {
        let shadows = parse_box_shadow("10px 10px 5px 2px #000");
        assert_eq!(shadows.len(), 1);
        assert_eq!(shadows[0].offset_x, 10.0);
        assert_eq!(shadows[0].offset_y, 10.0);
    }

    #[test]
    fn test_filter_parse() {
        let filters = parse_css_filter("blur(5px) brightness(1.5)");
        assert_eq!(filters.len(), 2);
    }
}
