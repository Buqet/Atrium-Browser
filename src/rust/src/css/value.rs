




use std::borrow::Cow;
use std::collections::HashMap;



#[derive(Clone, Debug)]
pub enum CssValue {
    
    String(Cow<'static, str>),
    
    Number(f32),
    
    Color(Color),
    
    Keyword(Cow<'static, str>),
    
    Url(Cow<'static, str>),
    
    None,
    
    Auto,
    
    Inherit,
    
    Initial,
    
    Unset,
    
    Revert,
    
    Length(CssLength),
    
    Calc(CssCalcExpression),
}

impl CssValue {
    
    pub fn is_custom_property(&self) -> bool {
        matches!(self, CssValue::String(s) if s.starts_with("--"))
    }

    
    pub fn to_px(&self, context: &ViewportContext) -> Option<f32> {
        match self {
            CssValue::Number(n) => Some(*n),
            CssValue::Length(len) => Some(len.to_px(context)),
            CssValue::Calc(expr) => Some(expr.evaluate(context)),
            CssValue::Keyword(k) if k.parse::<f32>().is_ok() => Some(k.parse().unwrap()),
            CssValue::String(s) if s.parse::<f32>().is_ok() => Some(s.parse().unwrap()),
            _ => None,
        }
    }
}







#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        match hex.len() {
            3 => {
                let chars: Vec<char> = hex.chars().collect();
                Some(Self {
                    r: u8::from_str_radix(&format!("{}{}", chars[0], chars[0]), 16).ok()?,
                    g: u8::from_str_radix(&format!("{}{}", chars[1], chars[1]), 16).ok()?,
                    b: u8::from_str_radix(&format!("{}{}", chars[2], chars[2]), 16).ok()?,
                    a: 255,
                })
            }
            6 => {
                Some(Self {
                    r: u8::from_str_radix(&hex[0..2], 16).ok()?,
                    g: u8::from_str_radix(&hex[2..4], 16).ok()?,
                    b: u8::from_str_radix(&hex[4..6], 16).ok()?,
                    a: 255,
                })
            }
            8 => {
                Some(Self {
                    r: u8::from_str_radix(&hex[0..2], 16).ok()?,
                    g: u8::from_str_radix(&hex[2..4], 16).ok()?,
                    b: u8::from_str_radix(&hex[4..6], 16).ok()?,
                    a: u8::from_str_radix(&hex[6..8], 16).ok()?,
                })
            }
            _ => None,
        }
    }

    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    
    
    pub fn from_hsl(h: f32, s: f32, l: f32, alpha: f32) -> Self {
        let h = ((h % 360.0) + 360.0) % 360.0;
        let s = s.clamp(0.0, 100.0) / 100.0;
        let l = l.clamp(0.0, 100.0) / 100.0;
        let a = (alpha * 255.0).clamp(0.0, 255.0).round() as u8;

        if s == 0.0 {
            let v = (l * 255.0).round() as u8;
            return Self::new(v, v, v, a);
        }

        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;

        let (r1, g1, b1) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        Self::new(
            ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
            ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
            ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
            a,
        )
    }

    
    
    pub fn from_hwb(h: f32, w: f32, b_: f32, alpha: f32) -> Self {
        
        let w_norm = w.clamp(0.0, 100.0) / 100.0;
        let b_norm = b_.clamp(0.0, 100.0) / 100.0;
        
        if w_norm + b_norm >= 1.0 {
            let gray = (w_norm / (w_norm + b_norm) * 255.0).round() as u8;
            let a = (alpha * 255.0).clamp(0.0, 255.0).round() as u8;
            return Self::new(gray, gray, gray, a);
        }

        let l = (1.0 - b_norm) * 100.0;
        let s = if l > 0.0 && l < 100.0 {
            (1.0 - w_norm / (1.0 - b_norm)) * 100.0
        } else {
            0.0
        };

        Self::from_hsl(h, s, l, alpha)
    }

    
    
    pub fn from_lab(l: f32, a: f32, b_: f32, alpha: f32) -> Self {
        
        let fy = (l + 16.0) / 116.0;
        let fx = a / 500.0 + fy;
        let fz = fy - b_ / 200.0;

        let delta = 6.0 / 29.0;
        let delta3 = delta * delta * delta;

        let xr = if fx > delta { fx * fx * fx } else { 3.0 * delta * delta * (fx - 4.0 / 29.0) };
        let yr = if fy > delta { fy * fy * fy } else { 3.0 * delta * delta * (fy - 4.0 / 29.0) };
        let zr = if fz > delta { fz * fz * fz } else { 3.0 * delta * delta * (fz - 4.0 / 29.0) };

        
        let x = xr * 95.047;
        let y = yr * 100.0;
        let z = zr * 108.883;

        Self::from_xyz_to_srgb(x, y, z, alpha)
    }

    
    
    pub fn from_lch(l: f32, c: f32, h: f32, alpha: f32) -> Self {
        let h_rad = h * std::f32::consts::PI / 180.0;
        let a = c * h_rad.cos();
        let b_ = c * h_rad.sin();
        Self::from_lab(l, a, b_, alpha)
    }

    
    
    pub fn from_oklab(l: f32, a: f32, b_: f32, alpha: f32) -> Self {
        
        let l_ = l + 0.3963377774 * a + 0.2158037573 * b_;
        let m_ = l - 0.1055613458 * a - 0.0638541728 * b_;
        let s_ = l - 0.0894841775 * a - 1.2914855480 * b_;

        let l_cubed = l_ * l_ * l_;
        let m_cubed = m_ * m_ * m_;
        let s_cubed = s_ * s_ * s_;

        
        let r_lin = 4.0767416621 * l_cubed - 3.3077115913 * m_cubed + 0.2309699292 * s_cubed;
        let g_lin = -1.2684380046 * l_cubed + 2.6097574011 * m_cubed - 0.3413193965 * s_cubed;
        let b_lin = -0.0041960863 * l_cubed - 0.7034186147 * m_cubed + 1.7076147010 * s_cubed;

        Self::from_linear_rgb(r_lin, g_lin, b_lin, alpha)
    }

    
    
    pub fn from_oklch(l: f32, c: f32, h: f32, alpha: f32) -> Self {
        let h_rad = h * std::f32::consts::PI / 180.0;
        let a = c * h_rad.cos();
        let b_ = c * h_rad.sin();
        Self::from_oklab(l, a, b_, alpha)
    }

    
    
    pub fn from_color_space(color_space: &str, components: &[f32], alpha: f32) -> Self {
        match color_space.to_lowercase().as_str() {
            "srgb" | "" => {
                if components.len() >= 3 {
                    let r = (components[0].clamp(0.0, 1.0) * 255.0).round() as u8;
                    let g = (components[1].clamp(0.0, 1.0) * 255.0).round() as u8;
                    let b = (components[2].clamp(0.0, 1.0) * 255.0).round() as u8;
                    let a = (alpha * 255.0).clamp(0.0, 255.0).round() as u8;
                    Self::new(r, g, b, a)
                } else {
                    Self::new(0, 0, 0, 255)
                }
            }
            "display-p3" => {
                
                if components.len() >= 3 {
                    let r_lin = components[0];
                    let g_lin = components[1];
                    let b_lin = components[2];
                    
                    let r = (r_lin.clamp(0.0, 1.0) * 255.0).round() as u8;
                    let g = (g_lin.clamp(0.0, 1.0) * 255.0).round() as u8;
                    let b = (b_lin.clamp(0.0, 1.0) * 255.0).round() as u8;
                    let a = (alpha * 255.0).clamp(0.0, 255.0).round() as u8;
                    Self::new(r, g, b, a)
                } else {
                    Self::new(0, 0, 0, 255)
                }
            }
            "xyz" | "xyz-d65" => {
                if components.len() >= 3 {
                    Self::from_xyz_to_srgb(components[0] * 100.0, components[1] * 100.0, components[2] * 100.0, alpha)
                } else {
                    Self::new(0, 0, 0, 255)
                }
            }
            _ => {
                
                if components.len() >= 3 {
                    let r = (components[0].clamp(0.0, 1.0) * 255.0).round() as u8;
                    let g = (components[1].clamp(0.0, 1.0) * 255.0).round() as u8;
                    let b = (components[2].clamp(0.0, 1.0) * 255.0).round() as u8;
                    let a = (alpha * 255.0).clamp(0.0, 255.0).round() as u8;
                    Self::new(r, g, b, a)
                } else {
                    Self::new(0, 0, 0, 255)
                }
            }
        }
    }

    

    
    fn from_xyz_to_srgb(x: f32, y: f32, z: f32, alpha: f32) -> Self {
        
        let r_lin = 3.2404542 * x - 1.5371385 * y - 0.4985314 * z;
        let g_lin = -0.9692660 * x + 1.8760108 * y + 0.0415560 * z;
        let b_lin = 0.0556434 * x - 0.2040259 * y + 1.0572252 * z;

        Self::from_linear_rgb(r_lin, g_lin, b_lin, alpha)
    }

    
    fn from_linear_rgb(r: f32, g: f32, b: f32, alpha: f32) -> Self {
        fn gamma(v: f32) -> f32 {
            if v <= 0.0031308 {
                (12.92 * v * 255.0).clamp(0.0, 255.0).round() as f32
            } else {
                ((1.055 * v.powf(1.0 / 2.4) - 0.055) * 255.0).clamp(0.0, 255.0).round() as f32
            }
        }

        Self::new(
            gamma(r) as u8,
            gamma(g) as u8,
            gamma(b) as u8,
            (alpha * 255.0).clamp(0.0, 255.0).round() as u8,
        )
    }

    
    
    pub fn current_color() -> Self {
        Self::new(0, 0, 0, 0) 
    }

    
    
    
    pub fn color_mix(color1: &Self, color2: &Self, mix_pct: f32) -> Self {
        let t = mix_pct.clamp(0.0, 1.0);
        Self::new(
            ((1.0 - t) * color1.r as f32 + t * color2.r as f32).round() as u8,
            ((1.0 - t) * color1.g as f32 + t * color2.g as f32).round() as u8,
            ((1.0 - t) * color1.b as f32 + t * color2.b as f32).round() as u8,
            ((1.0 - t) * color1.a as f32 + t * color2.a as f32).round() as u8,
        )
    }

    
    pub fn named(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "black" => Some(Self::new(0, 0, 0, 255)),
            "white" => Some(Self::new(255, 255, 255, 255)),
            "red" => Some(Self::new(255, 0, 0, 255)),
            "green" => Some(Self::new(0, 128, 0, 255)),
            "blue" => Some(Self::new(0, 0, 255, 255)),
            "yellow" => Some(Self::new(255, 255, 0, 255)),
            "cyan" => Some(Self::new(0, 255, 255, 255)),
            "magenta" => Some(Self::new(255, 0, 255, 255)),
            "gray" | "grey" => Some(Self::new(128, 128, 128, 255)),
            "silver" => Some(Self::new(192, 192, 192, 255)),
            "maroon" => Some(Self::new(128, 0, 0, 255)),
            "olive" => Some(Self::new(128, 128, 0, 255)),
            "lime" => Some(Self::new(0, 255, 0, 255)),
            "aqua" => Some(Self::new(0, 255, 255, 255)),
            "teal" => Some(Self::new(0, 128, 128, 255)),
            "navy" => Some(Self::new(0, 0, 128, 255)),
            "fuchsia" => Some(Self::new(255, 0, 255, 255)),
            "purple" => Some(Self::new(128, 0, 128, 255)),
            "orange" => Some(Self::new(255, 165, 0, 255)),
            "pink" => Some(Self::new(255, 192, 203, 255)),
            "brown" => Some(Self::new(165, 42, 42, 255)),
            "coral" => Some(Self::new(255, 127, 80, 255)),
            "crimson" => Some(Self::new(220, 20, 60, 255)),
            "gold" => Some(Self::new(255, 215, 0, 255)),
            "indigo" => Some(Self::new(75, 0, 130, 255)),
            "ivory" => Some(Self::new(255, 255, 240, 255)),
            "khaki" => Some(Self::new(240, 230, 140, 255)),
            "lavender" => Some(Self::new(230, 230, 250, 255)),
            "linen" => Some(Self::new(250, 240, 230, 255)),
            "mintcream" => Some(Self::new(245, 255, 250, 255)),
            "orchid" => Some(Self::new(218, 112, 214, 255)),
            "peru" => Some(Self::new(205, 133, 63, 255)),
            "plum" => Some(Self::new(221, 160, 221, 255)),
            "salmon" => Some(Self::new(250, 128, 114, 255)),
            "sienna" => Some(Self::new(160, 82, 45, 255)),
            "skyblue" => Some(Self::new(135, 206, 235, 255)),
            "snow" => Some(Self::new(255, 250, 250, 255)),
            "tan" => Some(Self::new(210, 180, 140, 255)),
            "thistle" => Some(Self::new(216, 191, 216, 255)),
            "tomato" => Some(Self::new(255, 99, 71, 255)),
            "turquoise" => Some(Self::new(64, 224, 208, 255)),
            "violet" => Some(Self::new(238, 130, 238, 255)),
            "wheat" => Some(Self::new(245, 222, 179, 255)),
            "whitesmoke" => Some(Self::new(245, 245, 245, 255)),
            "rebeccapurple" => Some(Self::new(102, 51, 153, 255)),
            "transparent" => Some(Self::new(0, 0, 0, 0)),
            "currentcolor" => Some(Self::current_color()),
            _ => None,
        }
    }

    
    pub fn to_hsl(&self) -> (f32, f32, f32, f32) {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;
        let a = self.a as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;

        if max == min {
            return (0.0, 0.0, l * 100.0, a);
        }

        let d = max - min;
        let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };

        let h = if max == r {
            (g - b) / d + (if g < b { 6.0 } else { 0.0 })
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };

        (h * 60.0, s * 100.0, l * 100.0, a)
    }
}





#[derive(Clone, Debug, PartialEq)]
pub enum CssLength {
    
    
    Px(f32),
    
    Cm(f32),
    
    Mm(f32),
    
    Q(f32),
    
    In(f32),
    
    Pc(f32),
    
    Pt(f32),
    
    
    
    Em(f32),
    
    Rem(f32),
    
    Ex(f32),
    
    Ch(f32),
    
    Cap(f32),
    
    Ic(f32),
    
    Lh(f32),
    
    Rlh(f32),
    
    
    
    Vw(f32),
    
    Vh(f32),
    
    Vmin(f32),
    
    Vmax(f32),
    
    Vi(f32),
    
    Vb(f32),
    
    
    
    Percent(f32),
    
    Number(f32),
    
    
    
    Deg(f32),
    
    Grad(f32),
    
    Rad(f32),
    
    Turn(f32),
    
    
    
    S(f32),
    
    Ms(f32),
    
    
    
    Hz(f32),
    
    KHz(f32),
    
    
    
    Dpi(f32),
    
    Dpcm(f32),
    
    Dppx(f32),
    
    
    
    Unknown(f32, String),
}

impl CssLength {
    
    pub fn to_px(&self, context: &ViewportContext) -> f32 {
        match self {
            
            CssLength::Px(v) => *v,
            CssLength::Cm(v) => v * 96.0 / 2.54,          
            CssLength::Mm(v) => v * 96.0 / 25.4,           
            CssLength::Q(v) => v * 96.0 / 40.6,            
            CssLength::In(v) => v * 96.0,                  
            CssLength::Pc(v) => v * 16.0,                  
            CssLength::Pt(v) => v * 96.0 / 72.0,           
            
            
            CssLength::Em(v) => v * context.font_size,
            CssLength::Rem(v) => v * context.root_font_size,
            CssLength::Ex(v) => v * context.font_size * 0.5,  
            CssLength::Ch(v) => v * context.font_size * 0.5,  
            CssLength::Cap(v) => v * context.font_size,        
            CssLength::Ic(v) => v * context.font_size,         
            CssLength::Lh(v) => v * context.font_size * 1.2,  
            CssLength::Rlh(v) => v * context.root_font_size * 1.2,
            
            
            CssLength::Vw(v) => v * context.viewport_width / 100.0,
            CssLength::Vh(v) => v * context.viewport_height / 100.0,
            CssLength::Vmin(v) => v * context.viewport_width.min(context.viewport_height) / 100.0,
            CssLength::Vmax(v) => v * context.viewport_width.max(context.viewport_height) / 100.0,
            CssLength::Vi(v) => v * context.viewport_width / 100.0,   
            CssLength::Vb(v) => v * context.viewport_height / 100.0,   
            
            
            CssLength::Percent(v) => {
                if let Some(containing_block) = context.containing_block_px {
                    *v / 100.0 * containing_block
                } else {
                    
                    *v
                }
            }
            
            
            CssLength::Number(v) => *v,
            
            
            CssLength::Deg(v) => *v,
            CssLength::Grad(v) => *v * 0.9,           
            CssLength::Rad(v) => v * 180.0 / std::f32::consts::PI,  
            CssLength::Turn(v) => v * 360.0,          
            
            
            CssLength::S(v) => v * 1000.0,             
            CssLength::Ms(v) => *v,
            
            
            CssLength::Hz(v) => *v,
            CssLength::KHz(v) => v * 1000.0,
            
            
            CssLength::Dpi(v) => v / 96.0,             
            CssLength::Dpcm(v) => v * 2.54 / 96.0,     
            CssLength::Dppx(v) => *v,
            
            
            CssLength::Unknown(v, _) => *v,
        }
    }

    
    pub fn value(&self) -> f32 {
        match self {
            CssLength::Px(v) | CssLength::Cm(v) | CssLength::Mm(v) | CssLength::Q(v)
            | CssLength::In(v) | CssLength::Pc(v) | CssLength::Pt(v)
            | CssLength::Em(v) | CssLength::Rem(v) | CssLength::Ex(v) | CssLength::Ch(v)
            | CssLength::Cap(v) | CssLength::Ic(v) | CssLength::Lh(v) | CssLength::Rlh(v)
            | CssLength::Vw(v) | CssLength::Vh(v) | CssLength::Vmin(v) | CssLength::Vmax(v)
            | CssLength::Vi(v) | CssLength::Vb(v)
            | CssLength::Percent(v) | CssLength::Number(v)
            | CssLength::Deg(v) | CssLength::Grad(v) | CssLength::Rad(v) | CssLength::Turn(v)
            | CssLength::S(v) | CssLength::Ms(v)
            | CssLength::Hz(v) | CssLength::KHz(v)
            | CssLength::Dpi(v) | CssLength::Dpcm(v) | CssLength::Dppx(v)
            | CssLength::Unknown(v, _) => *v,
        }
    }
    
    
    pub fn unit_name(&self) -> String {
        match self {
            CssLength::Px(_) => "px".to_string(),
            CssLength::Cm(_) => "cm".to_string(),
            CssLength::Mm(_) => "mm".to_string(),
            CssLength::Q(_) => "Q".to_string(),
            CssLength::In(_) => "in".to_string(),
            CssLength::Pc(_) => "pc".to_string(),
            CssLength::Pt(_) => "pt".to_string(),
            CssLength::Em(_) => "em".to_string(),
            CssLength::Rem(_) => "rem".to_string(),
            CssLength::Ex(_) => "ex".to_string(),
            CssLength::Ch(_) => "ch".to_string(),
            CssLength::Cap(_) => "cap".to_string(),
            CssLength::Ic(_) => "ic".to_string(),
            CssLength::Lh(_) => "lh".to_string(),
            CssLength::Rlh(_) => "rlh".to_string(),
            CssLength::Vw(_) => "vw".to_string(),
            CssLength::Vh(_) => "vh".to_string(),
            CssLength::Vmin(_) => "vmin".to_string(),
            CssLength::Vmax(_) => "vmax".to_string(),
            CssLength::Vi(_) => "vi".to_string(),
            CssLength::Vb(_) => "vb".to_string(),
            CssLength::Percent(_) => "%".to_string(),
            CssLength::Number(_) => String::new(),
            CssLength::Deg(_) => "deg".to_string(),
            CssLength::Grad(_) => "grad".to_string(),
            CssLength::Rad(_) => "rad".to_string(),
            CssLength::Turn(_) => "turn".to_string(),
            CssLength::S(_) => "s".to_string(),
            CssLength::Ms(_) => "ms".to_string(),
            CssLength::Hz(_) => "Hz".to_string(),
            CssLength::KHz(_) => "kHz".to_string(),
            CssLength::Dpi(_) => "dpi".to_string(),
            CssLength::Dpcm(_) => "dpcm".to_string(),
            CssLength::Dppx(_) => "dppx".to_string(),
            CssLength::Unknown(_, u) => u.clone(),
        }
    }
    
    
    pub fn from_value_and_unit(value: f32, unit: &str) -> Self {
        match unit {
            "px" => CssLength::Px(value),
            "cm" => CssLength::Cm(value),
            "mm" => CssLength::Mm(value),
            "Q" | "q" => CssLength::Q(value),
            "in" => CssLength::In(value),
            "pc" => CssLength::Pc(value),
            "pt" => CssLength::Pt(value),
            "em" => CssLength::Em(value),
            "rem" => CssLength::Rem(value),
            "ex" => CssLength::Ex(value),
            "ch" => CssLength::Ch(value),
            "cap" => CssLength::Cap(value),
            "ic" => CssLength::Ic(value),
            "lh" => CssLength::Lh(value),
            "rlh" => CssLength::Rlh(value),
            "vw" => CssLength::Vw(value),
            "vh" => CssLength::Vh(value),
            "vmin" => CssLength::Vmin(value),
            "vmax" => CssLength::Vmax(value),
            "vi" => CssLength::Vi(value),
            "vb" => CssLength::Vb(value),
            "%" => CssLength::Percent(value),
            "deg" => CssLength::Deg(value),
            "grad" => CssLength::Grad(value),
            "rad" => CssLength::Rad(value),
            "turn" => CssLength::Turn(value),
            "s" => CssLength::S(value),
            "ms" => CssLength::Ms(value),
            "Hz" | "hz" => CssLength::Hz(value),
            "kHz" | "khz" => CssLength::KHz(value),
            "dpi" => CssLength::Dpi(value),
            "dpcm" => CssLength::Dpcm(value),
            "dppx" => CssLength::Dppx(value),
            _ => CssLength::Unknown(value, unit.to_string()),
        }
    }
}





#[derive(Clone, Debug, Copy)]
pub struct ViewportContext {
    
    pub viewport_width: f32,
    
    pub viewport_height: f32,
    
    pub font_size: f32,
    
    pub root_font_size: f32,
    
    pub containing_block_px: Option<f32>,
}

impl ViewportContext {
    
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        Self {
            viewport_width,
            viewport_height,
            font_size: 16.0,
            root_font_size: 16.0,
            containing_block_px: None,
        }
    }
}


#[derive(Clone, Debug)]
pub enum CssCalcExpression {
    
    Add(Box<CssCalcExpression>, Box<CssCalcExpression>),
    
    Sub(Box<CssCalcExpression>, Box<CssCalcExpression>),
    
    Mul(Box<CssCalcExpression>, Box<CssCalcExpression>),
    
    Div(Box<CssCalcExpression>, Box<CssCalcExpression>),
    
    Length(CssLength),
    
    
    
    Abs(Box<CssCalcExpression>),
    
    Sign(Box<CssCalcExpression>),
    
    Round(Box<CssCalcExpression>, Box<CssCalcExpression>),
    
    Mod(Box<CssCalcExpression>, Box<CssCalcExpression>),
    
    Rem(Box<CssCalcExpression>, Box<CssCalcExpression>),
    
    Min(Vec<CssCalcExpression>),
    
    Max(Vec<CssCalcExpression>),
    
    Clamp {
        min: Box<CssCalcExpression>,
        val: Box<CssCalcExpression>,
        max: Box<CssCalcExpression>,
    },
    
    
    
    Sin(Box<CssCalcExpression>),
    
    Cos(Box<CssCalcExpression>),
    
    Tan(Box<CssCalcExpression>),
    
    Asin(Box<CssCalcExpression>),
    
    Acos(Box<CssCalcExpression>),
    
    Atan(Box<CssCalcExpression>),
    
    Atan2(Box<CssCalcExpression>, Box<CssCalcExpression>),
    
    
    
    Exp(Box<CssCalcExpression>),
    
    Log(Box<CssCalcExpression>),
    
    Pow(Box<CssCalcExpression>, Box<CssCalcExpression>),
    
    Sqrt(Box<CssCalcExpression>),
    
    Hypot(Box<CssCalcExpression>, Box<CssCalcExpression>),
}

impl CssCalcExpression {
    
    pub fn evaluate(&self, context: &ViewportContext) -> f32 {
        match self {
            CssCalcExpression::Add(a, b) => a.evaluate(context) + b.evaluate(context),
            CssCalcExpression::Sub(a, b) => a.evaluate(context) - b.evaluate(context),
            CssCalcExpression::Mul(a, b) => a.evaluate(context) * b.evaluate(context),
            CssCalcExpression::Div(a, b) => {
                let divisor = b.evaluate(context);
                if divisor != 0.0 {
                    a.evaluate(context) / divisor
                } else {
                    0.0
                }
            }
            CssCalcExpression::Length(len) => len.to_px(context),
            
            
            CssCalcExpression::Abs(e) => e.evaluate(context).abs(),
            CssCalcExpression::Sign(e) => {
                let v = e.evaluate(context);
                if v > 0.0 { 1.0 } else if v < 0.0 { -1.0 } else { 0.0 }
            }
            CssCalcExpression::Round(e, step) => {
                let v = e.evaluate(context);
                let s = step.evaluate(context);
                if s == 0.0 { v } else { (v / s).round() * s }
            }
            CssCalcExpression::Mod(e, m) => {
                let v = e.evaluate(context);
                let mod_val = m.evaluate(context);
                if mod_val == 0.0 { v } else { v.rem_euclid(mod_val) }
            }
            CssCalcExpression::Rem(e, m) => {
                let v = e.evaluate(context);
                let rem_val = m.evaluate(context);
                if rem_val == 0.0 { v } else { v % rem_val }
            }
            CssCalcExpression::Min(exprs) => {
                exprs.iter().map(|e| e.evaluate(context)).fold(f32::INFINITY, f32::min)
            }
            CssCalcExpression::Max(exprs) => {
                exprs.iter().map(|e| e.evaluate(context)).fold(f32::NEG_INFINITY, f32::max)
            }
            CssCalcExpression::Clamp { min, val, max } => {
                let v = val.evaluate(context);
                let min_v = min.evaluate(context);
                let max_v = max.evaluate(context);
                v.clamp(min_v, max_v)
            }
            
            
            CssCalcExpression::Sin(e) => {
                let deg = e.evaluate(context);
                let rad = deg * std::f32::consts::PI / 180.0;
                rad.sin()
            }
            CssCalcExpression::Cos(e) => {
                let deg = e.evaluate(context);
                let rad = deg * std::f32::consts::PI / 180.0;
                rad.cos()
            }
            CssCalcExpression::Tan(e) => {
                let deg = e.evaluate(context);
                let rad = deg * std::f32::consts::PI / 180.0;
                rad.tan()
            }
            CssCalcExpression::Asin(e) => {
                let v = e.evaluate(context).clamp(-1.0, 1.0);
                v.asin() * 180.0 / std::f32::consts::PI  
            }
            CssCalcExpression::Acos(e) => {
                let v = e.evaluate(context).clamp(-1.0, 1.0);
                v.acos() * 180.0 / std::f32::consts::PI  
            }
            CssCalcExpression::Atan(e) => {
                let v = e.evaluate(context);
                v.atan() * 180.0 / std::f32::consts::PI  
            }
            CssCalcExpression::Atan2(y, x) => {
                let y_val = y.evaluate(context);
                let x_val = x.evaluate(context);
                y_val.atan2(x_val) * 180.0 / std::f32::consts::PI  
            }
            
            
            CssCalcExpression::Exp(e) => e.evaluate(context).exp(),
            CssCalcExpression::Log(e) => {
                let v = e.evaluate(context);
                if v > 0.0 { v.ln() } else { 0.0 }
            }
            CssCalcExpression::Pow(base, exp) => {
                let b = base.evaluate(context);
                let e = exp.evaluate(context);
                b.powf(e)
            }
            CssCalcExpression::Sqrt(e) => {
                let v = e.evaluate(context);
                if v >= 0.0 { v.sqrt() } else { 0.0 }
            }
            CssCalcExpression::Hypot(x, y) => {
                let x_val = x.evaluate(context);
                let y_val = y.evaluate(context);
                (x_val * x_val + y_val * y_val).sqrt()
            }
        }
    }
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssDisplay {
    Block,
    Inline,
    InlineBlock,
    Flex,
    InlineFlex,
    Grid,
    InlineGrid,
    Table,
    TableRow,
    TableCell,
    TableHeaderGroup,
    TableRowGroup,
    TableFooterGroup,
    TableCaption,
    Contents,
    None,
    ListItem,
    RunIn,
}

impl CssDisplay {
    pub fn from_keyword(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "block" => CssDisplay::Block,
            "inline" => CssDisplay::Inline,
            "inline-block" => CssDisplay::InlineBlock,
            "flex" => CssDisplay::Flex,
            "inline-flex" => CssDisplay::InlineFlex,
            "grid" => CssDisplay::Grid,
            "inline-grid" => CssDisplay::InlineGrid,
            "table" => CssDisplay::Table,
            "table-row" => CssDisplay::TableRow,
            "table-cell" => CssDisplay::TableCell,
            "thead" | "table-header-group" => CssDisplay::TableHeaderGroup,
            "tbody" | "table-row-group" => CssDisplay::TableRowGroup,
            "tfoot" | "table-footer-group" => CssDisplay::TableFooterGroup,
            "caption" | "table-caption" => CssDisplay::TableCaption,
            "contents" => CssDisplay::Contents,
            "none" => CssDisplay::None,
            "list-item" => CssDisplay::ListItem,
            "run-in" => CssDisplay::RunIn,
            _ => CssDisplay::Inline, 
        }
    }
    
    pub fn is_flex(&self) -> bool {
        matches!(self, CssDisplay::Flex | CssDisplay::InlineFlex)
    }
    
    pub fn is_grid(&self) -> bool {
        matches!(self, CssDisplay::Grid | CssDisplay::InlineGrid)
    }
    
    pub fn is_inline(&self) -> bool {
        matches!(self, CssDisplay::Inline | CssDisplay::InlineBlock | CssDisplay::InlineFlex | CssDisplay::InlineGrid)
    }
    
    pub fn is_inline_block(&self) -> bool {
        matches!(self, CssDisplay::InlineBlock)
    }
    
    pub fn is_none(&self) -> bool {
        matches!(self, CssDisplay::None)
    }
    
    pub fn is_table(&self) -> bool {
        matches!(self, CssDisplay::Table | CssDisplay::TableRow | CssDisplay::TableCell
            | CssDisplay::TableHeaderGroup | CssDisplay::TableRowGroup | CssDisplay::TableFooterGroup
            | CssDisplay::TableCaption)
    }
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssPosition {
    Static,
    Relative,
    Absolute,
    Fixed,
    Sticky,
}

impl CssPosition {
    pub fn from_keyword(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "relative" => CssPosition::Relative,
            "absolute" => CssPosition::Absolute,
            "fixed" => CssPosition::Fixed,
            "sticky" => CssPosition::Sticky,
            _ => CssPosition::Static,
        }
    }
    
    pub fn is_positioned(&self) -> bool {
        !matches!(self, CssPosition::Static)
    }
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssFloat {
    Left,
    Right,
    None,
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssClear {
    Left,
    Right,
    Both,
    None,
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssBoxSizing {
    ContentBox,
    BorderBox,
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssTextAlign {
    Left,
    Right,
    Center,
    Justify,
    None,
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssWhiteSpace {
    Normal,
    Nowrap,
    Pre,
    PreWrap,
    PreLine,
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssOverflow {
    Visible,
    Hidden,
    Scroll,
    Auto,
    Clip,
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssFontStyle {
    Normal,
    Italic,
    Oblique,
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssTextTransform {
    None,
    Capitalize,
    Uppercase,
    Lowercase,
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssTextDecoration {
    None,
    Underline,
    Overline,
    LineThrough,
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssFlexDirection {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssFlexWrap {
    Nowrap,
    Wrap,
    WrapReverse,
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssJustifyContent {
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssAlignItems {
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssAlignSelf {
    Auto,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssVisibility {
    Visible,
    Hidden,
    Collapse,
}


#[derive(Clone, Debug)]
pub enum BackgroundImage {
    None,
    Url(String),
    Gradient(String),
}


#[derive(Clone, Debug)]
pub struct GridTrack {
    pub raw: String,
}









#[derive(Clone, Debug)]
pub struct ComputedStyle {
    
    pub display: CssDisplay,
    pub position: CssPosition,
    pub float: CssFloat,
    pub clear: CssClear,
    
    
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub min_width: Option<f32>,
    pub min_height: Option<f32>,
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,
    pub margin_top: f32,
    pub margin_right: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,
    pub border_top_width: f32,
    pub border_right_width: f32,
    pub border_bottom_width: f32,
    pub border_left_width: f32,
    pub box_sizing: CssBoxSizing,
    
    
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
    pub left: Option<f32>,
    pub z_index: Option<i32>,
    
    
    pub font_family: Vec<String>,
    pub font_size: f32,
    pub font_weight: f32,
    pub font_style: CssFontStyle,
    pub line_height: Option<f32>,
    pub letter_spacing: Option<f32>,
    pub text_align: CssTextAlign,
    pub text_decoration: CssTextDecoration,
    pub text_transform: CssTextTransform,
    pub white_space: CssWhiteSpace,
    
    
    pub color: Color,
    pub background_color: Option<Color>,
    pub background_image: Vec<BackgroundImage>,
    pub opacity: f32,
    
    
    pub flex_direction: CssFlexDirection,
    pub flex_wrap: CssFlexWrap,
    pub justify_content: CssJustifyContent,
    pub align_items: CssAlignItems,
    pub align_self: CssAlignSelf,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Option<f32>,
    pub gap: f32,
    
    
    pub grid_template_columns: Vec<GridTrack>,
    pub grid_template_rows: Vec<GridTrack>,
    pub grid_column_start: Option<i32>,
    pub grid_column_end: Option<i32>,
    pub grid_row_start: Option<i32>,
    pub grid_row_end: Option<i32>,
    
    
    pub overflow_x: CssOverflow,
    pub overflow_y: CssOverflow,
    pub visibility: CssVisibility,
}

impl ComputedStyle {
    
    pub fn new() -> Self {
        Self {
            display: CssDisplay::Inline,
            position: CssPosition::Static,
            float: CssFloat::None,
            clear: CssClear::None,
            
            width: None,
            height: None,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            margin_top: 0.0,
            margin_right: 0.0,
            margin_bottom: 0.0,
            margin_left: 0.0,
            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,
            border_top_width: 0.0,
            border_right_width: 0.0,
            border_bottom_width: 0.0,
            border_left_width: 0.0,
            box_sizing: CssBoxSizing::ContentBox,
            
            top: None,
            right: None,
            bottom: None,
            left: None,
            z_index: None,
            
            font_family: vec!["sans-serif".to_string()],
            font_size: 16.0,
            font_weight: 400.0,
            font_style: CssFontStyle::Normal,
            line_height: None,
            letter_spacing: None,
            text_align: CssTextAlign::Left,
            text_decoration: CssTextDecoration::None,
            text_transform: CssTextTransform::None,
            white_space: CssWhiteSpace::Normal,
            
            color: Color::new(0, 0, 0, 255),
            background_color: None,
            background_image: vec![BackgroundImage::None],
            opacity: 1.0,
            
            flex_direction: CssFlexDirection::Row,
            flex_wrap: CssFlexWrap::Nowrap,
            justify_content: CssJustifyContent::FlexStart,
            align_items: CssAlignItems::Stretch,
            align_self: CssAlignSelf::Auto,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: None,
            gap: 0.0,
            
            grid_template_columns: vec![],
            grid_template_rows: vec![],
            grid_column_start: None,
            grid_column_end: None,
            grid_row_start: None,
            grid_row_end: None,
            
            overflow_x: CssOverflow::Visible,
            overflow_y: CssOverflow::Visible,
            visibility: CssVisibility::Visible,
        }
    }
    
    
    pub fn from_style_map(styles: &rustc_hash::FxHashMap<String, CssValue>, context: &ViewportContext) -> Self {
        let mut computed = Self::new();
        
        
        if let Some(CssValue::Keyword(k)) = styles.get("display") {
            computed.display = CssDisplay::from_keyword(k);
        }
        
        
        if let Some(CssValue::Keyword(k)) = styles.get("position") {
            computed.position = CssPosition::from_keyword(k);
        }
        
        
        for (prop, field) in [
            ("margin-top", &mut computed.margin_top),
            ("margin-right", &mut computed.margin_right),
            ("margin-bottom", &mut computed.margin_bottom),
            ("margin-left", &mut computed.margin_left),
            ("padding-top", &mut computed.padding_top),
            ("padding-right", &mut computed.padding_right),
            ("padding-bottom", &mut computed.padding_bottom),
            ("padding-left", &mut computed.padding_left),
        ] {
            if let Some(val) = styles.get(prop) {
                if let Some(px) = val.to_px(context) {
                    *field = px;
                }
            }
        }
        
        
        for (prop, field) in [
            ("border-top-width", &mut computed.border_top_width),
            ("border-right-width", &mut computed.border_right_width),
            ("border-bottom-width", &mut computed.border_bottom_width),
            ("border-left-width", &mut computed.border_left_width),
        ] {
            if let Some(val) = styles.get(prop) {
                if let Some(px) = val.to_px(context) {
                    *field = px;
                }
            }
        }
        
        
        for (prop, field) in [
            ("width", &mut computed.width),
            ("height", &mut computed.height),
            ("min-width", &mut computed.min_width),
            ("min-height", &mut computed.min_height),
            ("max-width", &mut computed.max_width),
            ("max-height", &mut computed.max_height),
        ] {
            if let Some(val) = styles.get(prop) {
                *field = val.to_px(context);
            }
        }
        
        
        if let Some(val) = styles.get("font-size") {
            if let Some(px) = val.to_px(context) {
                computed.font_size = px;
            }
        }
        
        
        if let Some(CssValue::Color(c)) = styles.get("color") {
            computed.color = c.clone();
        }
        
        
        if let Some(CssValue::Color(c)) = styles.get("background-color") {
            computed.background_color = Some(c.clone());
        }
        
        
        if let Some(CssValue::Number(n)) = styles.get("opacity") {
            computed.opacity = (*n).clamp(0.0, 1.0);
        }
        
        
        for (prop, field) in [
            ("top", &mut computed.top),
            ("right", &mut computed.right),
            ("bottom", &mut computed.bottom),
            ("left", &mut computed.left),
        ] {
            if let Some(val) = styles.get(prop) {
                *field = val.to_px(context);
            }
        }
        
        
        if let Some(CssValue::Number(n)) = styles.get("z-index") {
            computed.z_index = Some(*n as i32);
        }
        
        computed
    }
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssVerticalAlign {
    Top,
    Middle,
    Bottom,
    Baseline,
    TextTop,
    TextBottom,
    Sub,
    Super,
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssListStyleType {
    Disc,
    Circle,
    Square,
    Decimal,
    DecimalLeadingZero,
    LowerRoman,
    UpperRoman,
    LowerGreek,
    LowerLatin,
    UpperLatin,
    None,
    Custom(String),
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssListStylePosition {
    Outside,
    Inside,
}


#[derive(Clone, Debug)]
pub enum CssTransform {
    TranslateX(f32),
    TranslateY(f32),
    Translate(f32, f32),
    Rotate(f32), 
    Scale(f32),
    ScaleX(f32),
    ScaleY(f32),
    SkewX(f32), 
    SkewY(f32), 
    Matrix(f32, f32, f32, f32, f32, f32),
}


#[derive(Clone, Debug)]
pub struct CssBoxShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
    pub spread_radius: f32,
    pub color: Option<Color>,
    pub inset: bool,
}


#[derive(Clone, Debug)]
pub enum CssFilter {
    Blur(f32), 
    Brightness(f32),
    Contrast(f32),
    Grayscale(f32),
    Sepia(f32),
    Saturate(f32),
    HueRotate(f32), 
    Invert(f32),
    Opacity(f32),
}


pub fn parse_css_float(value: &CssValue) -> CssFloat {
    match value {
        CssValue::Keyword(s) => match s.as_ref() {
            "left" => CssFloat::Left,
            "right" => CssFloat::Right,
            _ => CssFloat::None,
        },
        _ => CssFloat::None,
    }
}


pub fn parse_css_clear(value: &CssValue) -> CssClear {
    match value {
        CssValue::Keyword(s) => match s.as_ref() {
            "left" => CssClear::Left,
            "right" => CssClear::Right,
            "both" => CssClear::Both,
            _ => CssClear::None,
        },
        _ => CssClear::None,
    }
}


pub fn parse_css_box_sizing(value: &CssValue) -> CssBoxSizing {
    match value {
        CssValue::Keyword(s) => match s.as_ref() {
            "border-box" => CssBoxSizing::BorderBox,
            _ => CssBoxSizing::ContentBox,
        },
        _ => CssBoxSizing::ContentBox,
    }
}


pub fn parse_css_text_align(value: &CssValue) -> CssTextAlign {
    match value {
        CssValue::Keyword(s) => match s.as_ref() {
            "left" => CssTextAlign::Left,
            "right" => CssTextAlign::Right,
            "center" => CssTextAlign::Center,
            "justify" => CssTextAlign::Justify,
            _ => CssTextAlign::None,
        },
        _ => CssTextAlign::None,
    }
}


pub fn parse_css_vertical_align(value: &CssValue) -> CssVerticalAlign {
    match value {
        CssValue::Keyword(s) => match s.as_ref() {
            "top" => CssVerticalAlign::Top,
            "middle" => CssVerticalAlign::Middle,
            "bottom" => CssVerticalAlign::Bottom,
            "baseline" => CssVerticalAlign::Baseline,
            "text-top" => CssVerticalAlign::TextTop,
            "text-bottom" => CssVerticalAlign::TextBottom,
            "sub" => CssVerticalAlign::Sub,
            "super" => CssVerticalAlign::Super,
            _ => CssVerticalAlign::Baseline,
        },
        _ => CssVerticalAlign::Baseline,
    }
}


pub fn parse_css_white_space(value: &CssValue) -> CssWhiteSpace {
    match value {
        CssValue::Keyword(s) => match s.as_ref() {
            "normal" => CssWhiteSpace::Normal,
            "nowrap" => CssWhiteSpace::Nowrap,
            "pre" => CssWhiteSpace::Pre,
            "pre-wrap" => CssWhiteSpace::PreWrap,
            "pre-line" => CssWhiteSpace::PreLine,
            _ => CssWhiteSpace::Normal,
        },
        _ => CssWhiteSpace::Normal,
    }
}


pub fn parse_css_overflow(value: &CssValue) -> CssOverflow {
    match value {
        CssValue::Keyword(s) => match s.as_ref() {
            "visible" => CssOverflow::Visible,
            "hidden" => CssOverflow::Hidden,
            "scroll" => CssOverflow::Scroll,
            "auto" => CssOverflow::Auto,
            "clip" => CssOverflow::Clip,
            _ => CssOverflow::Visible,
        },
        _ => CssOverflow::Visible,
    }
}


pub fn parse_list_style_type(value: &str) -> CssListStyleType {
    match value.trim().to_lowercase().as_str() {
        "disc" => CssListStyleType::Disc,
        "circle" => CssListStyleType::Circle,
        "square" => CssListStyleType::Square,
        "decimal" => CssListStyleType::Decimal,
        "decimal-leading-zero" => CssListStyleType::DecimalLeadingZero,
        "lower-roman" => CssListStyleType::LowerRoman,
        "upper-roman" => CssListStyleType::UpperRoman,
        "lower-greek" => CssListStyleType::LowerGreek,
        "lower-latin" => CssListStyleType::LowerLatin,
        "upper-latin" => CssListStyleType::UpperLatin,
        "none" => CssListStyleType::None,
        other => CssListStyleType::Custom(other.to_string()),
    }
}


pub fn parse_list_style_position(value: &str) -> CssListStylePosition {
    match value.trim().to_lowercase().as_str() {
        "inside" => CssListStylePosition::Inside,
        _ => CssListStylePosition::Outside,
    }
}


pub fn parse_font_family(value: &str) -> Vec<String> {
    value.split(',')
        .map(|f| f.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|f| !f.is_empty())
        .collect()
}


pub fn parse_line_height(value: &str) -> Option<f32> {
    let value = value.trim();

    if let Ok(num) = value.parse::<f32>() {
        return Some(num);
    }

    let mut num_str = String::new();
    for c in value.chars() {
        if c.is_digit(10) || c == '.' || c == '-' {
            num_str.push(c);
        } else {
            break;
        }
    }

    if !num_str.is_empty() {
        return num_str.parse::<f32>().ok();
    }

    None
}


#[derive(Clone, Debug, PartialEq)]
pub enum CssObjectFit {
    Fill,
    Contain,
    Cover,
    None_,
    ScaleDown,
}

pub fn parse_object_fit(value: &str) -> CssObjectFit {
    match value.trim().to_lowercase().as_str() {
        "fill" => CssObjectFit::Fill,
        "contain" => CssObjectFit::Contain,
        "cover" => CssObjectFit::Cover,
        "none" => CssObjectFit::None_,
        "scale-down" => CssObjectFit::ScaleDown,
        _ => CssObjectFit::Fill,
    }
}


#[derive(Clone, Debug)]
pub enum CssClipPath {
    None_,
    Circle(f32, f32, f32), 
    Ellipse(f32, f32, f32, f32), 
    Inset(f32, f32, f32, f32), 
    Polygon(Vec<(f32, f32)>), 
}

pub fn parse_clip_path(value: &str) -> Option<CssClipPath> {
    let value = value.trim();

    if value.eq_ignore_ascii_case("none") {
        return Some(CssClipPath::None_);
    }

    if let Some(paren_pos) = value.find('(') {
        let func_name = value[..paren_pos].to_lowercase();
        if !value.ends_with(')') {
            return None;
        }
        let inner = &value[paren_pos + 1..value.len() - 1];

        match func_name.as_str() {
            "circle" => {
                let parts: Vec<f32> = inner.split_whitespace()
                    .filter_map(|s| {
                        let mut num = String::new();
                        for c in s.chars() {
                            if c.is_digit(10) || c == '.' || c == '-' {
                                num.push(c);
                            } else {
                                break;
                            }
                        }
                        num.parse().ok()
                    })
                    .collect();
                if parts.len() >= 3 {
                    Some(CssClipPath::Circle(parts[0], parts[1], parts[2]))
                } else if let Some(r) = parts.first() {
                    Some(CssClipPath::Circle(*r, 50.0, 50.0))
                } else {
                    None
                }
            }
            "ellipse" => {
                let parts: Vec<f32> = inner.split_whitespace()
                    .filter_map(|s| {
                        let mut num = String::new();
                        for c in s.chars() {
                            if c.is_digit(10) || c == '.' || c == '-' {
                                num.push(c);
                            } else {
                                break;
                            }
                        }
                        num.parse().ok()
                    })
                    .collect();
                if parts.len() >= 4 {
                    Some(CssClipPath::Ellipse(parts[0], parts[1], parts[2], parts[3]))
                } else {
                    None
                }
            }
            "inset" => {
                let parts: Vec<f32> = inner.split_whitespace()
                    .filter_map(|s| {
                        let mut num = String::new();
                        for c in s.chars() {
                            if c.is_digit(10) || c == '.' || c == '-' {
                                num.push(c);
                            } else {
                                break;
                            }
                        }
                        num.parse().ok()
                    })
                    .collect();
                if parts.len() == 4 {
                    Some(CssClipPath::Inset(parts[0], parts[1], parts[2], parts[3]))
                } else {
                    None
                }
            }
            "polygon" => {
                let mut points = Vec::new();
                for pair in inner.split(',') {
                    let nums: Vec<f32> = pair.split_whitespace()
                        .filter_map(|s| {
                            let mut num = String::new();
                            for c in s.chars() {
                                if c.is_digit(10) || c == '.' || c == '-' {
                                    num.push(c);
                                } else {
                                    break;
                                }
                            }
                            num.parse().ok()
                        })
                        .collect();
                    if nums.len() == 2 {
                        points.push((nums[0], nums[1]));
                    }
                }
                if !points.is_empty() {
                    Some(CssClipPath::Polygon(points))
                } else {
                    None
                }
            }
            _ => None,
        }
    } else {
        None
    }
}


#[derive(Clone, Debug, Default)]
pub struct PseudoElement {
    pub content: Option<String>,
    pub styles: HashMap<String, CssValue>,
}


#[cfg(feature = "egui")]
pub fn css_to_egui_color(value: &CssValue) -> Option<egui::Color32> {
    match value {
        CssValue::Color(color) => Some(egui::Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)),
        CssValue::Keyword(name) => {
            if let Some(color) = Color::named(name) {
                Some(egui::Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a))
            } else {
                None
            }
        }
        _ => None,
    }
}


#[cfg(not(feature = "egui"))]
pub fn css_to_egui_color(_value: &CssValue) -> Option<()> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_from_hex() {
        assert!(Color::from_hex("#fff").is_some());
        assert!(Color::from_hex("#ffffff").is_some());
        assert!(Color::from_hex("#ffffffff").is_some());
        assert!(Color::from_hex("#f").is_none());
    }

    #[test]
    fn test_css_length_to_px() {
        let mut ctx = ViewportContext::new(1920.0, 1080.0);
        ctx.font_size = 16.0;
        ctx.root_font_size = 16.0;

        
        assert_eq!(CssLength::Px(100.0).to_px(&ctx), 100.0);
        assert!((CssLength::Cm(1.0).to_px(&ctx) - 37.795).abs() < 0.1);
        assert!((CssLength::In(1.0).to_px(&ctx) - 96.0).abs() < 0.01);
        assert!((CssLength::Pt(72.0).to_px(&ctx) - 96.0).abs() < 0.01);
        assert!((CssLength::Pc(6.0).to_px(&ctx) - 96.0).abs() < 0.01);
        
        
        assert_eq!(CssLength::Em(2.0).to_px(&ctx), 32.0);
        assert_eq!(CssLength::Rem(2.0).to_px(&ctx), 32.0);
        
        
        assert_eq!(CssLength::Vw(50.0).to_px(&ctx), 960.0);
        assert_eq!(CssLength::Vh(50.0).to_px(&ctx), 540.0);
        assert_eq!(CssLength::Vmin(10.0).to_px(&ctx), 108.0);
        assert_eq!(CssLength::Vmax(10.0).to_px(&ctx), 192.0);
        
        
        assert_eq!(CssLength::Deg(180.0).to_px(&ctx), 180.0);
        assert!((CssLength::Rad(1.0).to_px(&ctx) - 57.296).abs() < 0.1);
        assert_eq!(CssLength::Turn(1.0).to_px(&ctx), 360.0);
        
        
        assert_eq!(CssLength::S(1.0).to_px(&ctx), 1000.0);
        assert_eq!(CssLength::Ms(500.0).to_px(&ctx), 500.0);
        
        
        assert!((CssLength::Dpi(96.0).to_px(&ctx) - 1.0).abs() < 0.01);
        assert_eq!(CssLength::Dppx(2.0).to_px(&ctx), 2.0);
    }
    
    #[test]
    fn test_css_length_from_value_and_unit() {
        assert_eq!(CssLength::from_value_and_unit(100.0, "px"), CssLength::Px(100.0));
        assert_eq!(CssLength::from_value_and_unit(1.0, "cm"), CssLength::Cm(1.0));
        assert_eq!(CssLength::from_value_and_unit(90.0, "deg"), CssLength::Deg(90.0));
        assert_eq!(CssLength::from_value_and_unit(1.0, "s"), CssLength::S(1.0));
        assert_eq!(CssLength::from_value_and_unit(500.0, "ms"), CssLength::Ms(500.0));
        assert_eq!(CssLength::from_value_and_unit(96.0, "dpi"), CssLength::Dpi(96.0));
        assert!(matches!(CssLength::from_value_and_unit(100.0, "unknown"), CssLength::Unknown(100.0, _)));
    }
    
    #[test]
    fn test_css_calc_math_functions() {
        let mut ctx = ViewportContext::new(1920.0, 1080.0);
        ctx.font_size = 16.0;
        ctx.root_font_size = 16.0;
        
        
        let abs_expr = CssCalcExpression::Abs(Box::new(
            CssCalcExpression::Length(CssLength::Px(-100.0))
        ));
        assert_eq!(abs_expr.evaluate(&ctx), 100.0);
        
        
        let sqrt_expr = CssCalcExpression::Sqrt(Box::new(
            CssCalcExpression::Length(CssLength::Number(16.0))
        ));
        assert_eq!(sqrt_expr.evaluate(&ctx), 4.0);
        
        
        let sin_expr = CssCalcExpression::Sin(Box::new(
            CssCalcExpression::Length(CssLength::Deg(90.0))
        ));
        assert!((sin_expr.evaluate(&ctx) - 1.0).abs() < 0.001);
        
        let cos_expr = CssCalcExpression::Cos(Box::new(
            CssCalcExpression::Length(CssLength::Deg(0.0))
        ));
        assert!((cos_expr.evaluate(&ctx) - 1.0).abs() < 0.001);
        
        
        let min_expr = CssCalcExpression::Min(vec![
            CssCalcExpression::Length(CssLength::Px(100.0)),
            CssCalcExpression::Length(CssLength::Px(200.0)),
        ]);
        assert_eq!(min_expr.evaluate(&ctx), 100.0);
        
        let max_expr = CssCalcExpression::Max(vec![
            CssCalcExpression::Length(CssLength::Px(100.0)),
            CssCalcExpression::Length(CssLength::Px(200.0)),
        ]);
        assert_eq!(max_expr.evaluate(&ctx), 200.0);
        
        
        let clamp_expr = CssCalcExpression::Clamp {
            min: Box::new(CssCalcExpression::Length(CssLength::Px(50.0))),
            val: Box::new(CssCalcExpression::Length(CssLength::Px(100.0))),
            max: Box::new(CssCalcExpression::Length(CssLength::Px(75.0))),
        };
        assert_eq!(clamp_expr.evaluate(&ctx), 75.0);
        
        
        let pow_expr = CssCalcExpression::Pow(
            Box::new(CssCalcExpression::Length(CssLength::Number(2.0))),
            Box::new(CssCalcExpression::Length(CssLength::Number(3.0))),
        );
        assert_eq!(pow_expr.evaluate(&ctx), 8.0);
    }

    #[test]
    fn test_css_value_cow_borrow() {
        let v = CssValue::Keyword(Cow::Borrowed("auto"));
        match v {
            CssValue::Keyword(k) => assert_eq!(k, "auto"),
            _ => panic!("Expected Keyword"),
        }
    }

    #[test]
    fn test_css_value_cow_owned() {
        let s = String::from("bold");
        let v = CssValue::Keyword(Cow::Owned(s));
        match v {
            CssValue::Keyword(k) => assert_eq!(k, "bold"),
            _ => panic!("Expected Keyword"),
        }
    }

    #[test]
    fn test_color_from_hsl() {
        let red = Color::from_hsl(0.0, 100.0, 50.0, 1.0);
        assert_eq!(red.r, 255);
        assert_eq!(red.g, 0);
        assert_eq!(red.b, 0);
        
        let green = Color::from_hsl(120.0, 100.0, 50.0, 1.0);
        assert_eq!(green.g, 255);
        
        let gray = Color::from_hsl(0.0, 0.0, 50.0, 1.0);
        assert_eq!(gray.r, 128);
        assert_eq!(gray.g, 128);
        assert_eq!(gray.b, 128);
    }

    #[test]
    fn test_color_from_hwb() {
        let black = Color::from_hwb(0.0, 0.0, 100.0, 1.0);
        assert_eq!(black.r, 0);
        assert_eq!(black.g, 0);
        assert_eq!(black.b, 0);
        
        let white = Color::from_hwb(0.0, 100.0, 0.0, 1.0);
        assert_eq!(white.r, 255);
    }

    #[test]
    fn test_color_mix() {
        let red = Color::from_rgb(255, 0, 0);
        let blue = Color::from_rgb(0, 0, 255);
        
        let mix = Color::color_mix(&red, &blue, 0.5);
        assert_eq!(mix.r, 128);
        assert_eq!(mix.b, 128);
        
        let full = Color::color_mix(&red, &blue, 1.0);
        assert_eq!(full.r, 0);
        assert_eq!(full.b, 255);
    }

    #[test]
    fn test_color_named_extended() {
        assert!(Color::named("rebeccapurple").is_some());
        assert!(Color::named("coral").is_some());
        assert!(Color::named("currentcolor").is_some());
        assert!(Color::named("nonexistent_xyz").is_none());
    }
}
