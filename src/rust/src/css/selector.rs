use crate::html::HtmlNode;

#[derive(Clone, Debug, Default)]
pub struct ElementState {
    pub hover: bool,
    pub focus: bool,
    pub active: bool,
    pub checked: bool,
    pub disabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Specificity {
    pub id_count: u32,
    pub class_count: u32,
    pub type_count: u32,
}

impl Specificity {
    pub fn calculate(selector: &Selector) -> Self {
        match selector {
            Selector::Universal => Self::default(),
            Selector::Type(_) => Self { id_count: 0, class_count: 0, type_count: 1 },
            Selector::Class(_) => Self { id_count: 0, class_count: 1, type_count: 0 },
            Selector::Id(_) => Self { id_count: 1, class_count: 0, type_count: 0 },
            Selector::Attribute(_, _) => Self { id_count: 0, class_count: 1, type_count: 0 },
            Selector::PseudoClass(_) => Self { id_count: 0, class_count: 1, type_count: 0 },
            Selector::PseudoElement(_) => Self { id_count: 0, class_count: 0, type_count: 1 },
            Selector::Descendant(a, b) => {
                let spec_a = Self::calculate(a);
                let spec_b = Self::calculate(b);
                Self {
                    id_count: spec_a.id_count + spec_b.id_count,
                    class_count: spec_a.class_count + spec_b.class_count,
                    type_count: spec_a.type_count + spec_b.type_count,
                }
            }
            Selector::Child(a, b) => {
                let spec_a = Self::calculate(a);
                let spec_b = Self::calculate(b);
                Self {
                    id_count: spec_a.id_count + spec_b.id_count,
                    class_count: spec_a.class_count + spec_b.class_count,
                    type_count: spec_a.type_count + spec_b.type_count,
                }
            }
            Selector::Adjacent(a, b) => {
                let spec_a = Self::calculate(a);
                let spec_b = Self::calculate(b);
                Self {
                    id_count: spec_a.id_count + spec_b.id_count,
                    class_count: spec_a.class_count + spec_b.class_count,
                    type_count: spec_a.type_count + spec_b.type_count,
                }
            }
            Selector::GeneralSibling(a, b) => {
                let spec_a = Self::calculate(a);
                let spec_b = Self::calculate(b);
                Self {
                    id_count: spec_a.id_count + spec_b.id_count,
                    class_count: spec_a.class_count + spec_b.class_count,
                    type_count: spec_a.type_count + spec_b.type_count,
                }
            }
            Selector::Group(selectors) => {
                selectors.iter()
                    .map(Self::calculate)
                    .max()
                    .unwrap_or_default()
            }
            Selector::Not(inner) => {
                
                Self::calculate(inner)
            }
            Selector::NthChild(_, _) => Self { id_count: 0, class_count: 1, type_count: 0 },
        }
    }
}


#[derive(Clone, Debug)]
pub enum Selector {

    Ampersand,
    
    Universal,
    
    Type(String),
    
    Class(String),
    
    Id(String),
    
    
    Attribute(String, Option<(String, crate::css::matcher::AttributeMatchType)>),
    
    Descendant(Box<Selector>, Box<Selector>),
    
    Child(Box<Selector>, Box<Selector>),
    
    Adjacent(Box<Selector>, Box<Selector>),
    
    GeneralSibling(Box<Selector>, Box<Selector>),
    
    PseudoClass(String),
    
    PseudoElement(String),
    
    Group(Vec<Selector>),
    
    Not(Box<Selector>),
    
    NthChild(i32, i32), 
}





pub fn matches_selector(
    selector: &Selector,
    node: &HtmlNode,
    parent: Option<&HtmlNode>,
    siblings: Option<&[HtmlNode]>,
    sibling_index: usize,
    state: &ElementState,
) -> bool {
    match selector {
        Selector::Universal => true,
        Selector::Type(tag_name) => {
            if let HtmlNode::Element { tag, .. } = node {
                tag.to_lowercase() == tag_name.to_lowercase()
            } else {
                false
            }
        }
        Selector::Class(class_name) => {
            if let HtmlNode::Element { attributes, .. } = node {
                attributes.get("class")
                    .map(|c| c.split_whitespace().any(|c| c == class_name))
                    .unwrap_or(false)
            } else {
                false
            }
        }
        Selector::Id(id_name) => {
            if let HtmlNode::Element { attributes, .. } = node {
                attributes.get("id").map(|id| id == id_name).unwrap_or(false)
            } else {
                false
            }
        }
        Selector::Attribute(name, value) => {
            if let HtmlNode::Element { attributes, .. } = node {
                match value {
                    Some((val, match_type)) => {
                        if let Some(attr_val) = attributes.get(name) {
                            match match_type {
                                crate::css::matcher::AttributeMatchType::Exact => attr_val == val,
                                crate::css::matcher::AttributeMatchType::Includes => {
                                    attr_val.split_whitespace().any(|w| w == val)
                                }
                                crate::css::matcher::AttributeMatchType::DashMatch => {
                                    attr_val == val || attr_val.starts_with(&format!("{}-", val))
                                }
                                crate::css::matcher::AttributeMatchType::Prefix => {
                                    attr_val.starts_with(val)
                                }
                                crate::css::matcher::AttributeMatchType::Suffix => {
                                    attr_val.ends_with(val)
                                }
                                crate::css::matcher::AttributeMatchType::Substring => {
                                    attr_val.contains(val)
                                }
                            }
                        } else {
                            false
                        }
                    }
                    None => attributes.contains_key(name),
                }
            } else {
                false
            }
        }
        Selector::PseudoClass(name) => {
            match_pseudo_class(name, node, parent, siblings, sibling_index, state)
        }
        Selector::PseudoElement(_) => {
            
            
            false
        }
        Selector::Descendant(parent_sel, child_sel) => {
            if !matches_selector(child_sel, node, parent, siblings, sibling_index, state) {
                return false;
            }
            
            
            true
        }
        Selector::Child(parent_sel, child_sel) => {
            matches_selector(child_sel, node, parent, siblings, sibling_index, state) &&
            parent.map(|p| matches_selector(parent_sel, p, None, None, 0, state)).unwrap_or(false)
        }
        Selector::Adjacent(_, _) => false, 
        Selector::GeneralSibling(_, _) => false, 
        Selector::Group(selectors) => selectors.iter().any(|s| matches_selector(s, node, parent, siblings, sibling_index, state)),
        Selector::Not(inner) => !matches_selector(inner, node, parent, siblings, sibling_index, state),
        Selector::NthChild(a, b) => {
            if let Some(_sibs) = siblings {
                
                let n = sibling_index as i32 + 1;
                *a >= 0 && (n - *b) % *a == 0 && (n - *b) / *a >= 0
            } else {
                false
            }
        }
    }
}


fn match_pseudo_class(
    name: &str,
    node: &HtmlNode,
    parent: Option<&HtmlNode>,
    siblings: Option<&[HtmlNode]>,
    sibling_index: usize,
    state: &ElementState,
) -> bool {
    match name {
        
        "hover" => state.hover,
        "focus" => state.focus,
        "active" => state.active,

        
        "checked" => state.checked,
        "disabled" => state.disabled,

        
        "first-child" => {
            sibling_index == 0
        }
        "last-child" => {
            if let Some(sibs) = siblings {
                sibling_index == sibs.len() - 1
            } else {
                false
            }
        }
        "only-child" => {
            if let Some(sibs) = siblings {
                sibs.len() == 1
            } else {
                false
            }
        }
        "first-of-type" => {
            if let Some(sibs) = siblings {
                if let HtmlNode::Element { tag, .. } = node {
                    sibs.iter().take(sibling_index).all(|s| {
                        if let HtmlNode::Element { tag: s_tag, .. } = s {
                            s_tag != tag
                        } else {
                            true
                        }
                    })
                } else {
                    false
                }
            } else {
                false
            }
        }
        "last-of-type" => {
            if let Some(sibs) = siblings {
                if let HtmlNode::Element { tag, .. } = node {
                    sibs.iter().skip(sibling_index + 1).all(|s| {
                        if let HtmlNode::Element { tag: s_tag, .. } = s {
                            s_tag != tag
                        } else {
                            true
                        }
                    })
                } else {
                    false
                }
            } else {
                false
            }
        }
        "empty" => {
            if let HtmlNode::Element { children, .. } = node {
                children.is_empty()
            } else {
                false
            }
        }
        "root" => {
            
            if let HtmlNode::Element { tag, .. } = node {
                tag.to_lowercase() == "html" && parent.is_none()
            } else {
                false
            }
        }

        
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specificity() {
        let type_sel = Specificity::calculate(&Selector::Type("div".to_string()));
        assert_eq!(type_sel.type_count, 1);
        assert_eq!(type_sel.class_count, 0);
        assert_eq!(type_sel.id_count, 0);

        let class_sel = Specificity::calculate(&Selector::Class("foo".to_string()));
        assert_eq!(class_sel.type_count, 0);
        assert_eq!(class_sel.class_count, 1);
        assert_eq!(class_sel.id_count, 0);

        let id_sel = Specificity::calculate(&Selector::Id("bar".to_string()));
        assert_eq!(id_sel.type_count, 0);
        assert_eq!(id_sel.class_count, 0);
        assert_eq!(id_sel.id_count, 1);

        
        assert!(class_sel > type_sel);
        
        assert!(id_sel > class_sel);
    }

    #[test]
    fn test_specificity_combined() {
        
        let sel = Selector::Descendant(
            Box::new(Selector::Id("id".to_string())),
            Box::new(Selector::Descendant(
                Box::new(Selector::Class("class".to_string())),
                Box::new(Selector::Type("div".to_string())),
            )),
        );
        let spec = Specificity::calculate(&sel);
        assert_eq!(spec.id_count, 1);
        assert_eq!(spec.class_count, 1);
        assert_eq!(spec.type_count, 1);
    }

    #[test]
    fn test_specificity_not() {
        
        let sel = Selector::Not(Box::new(Selector::Id("id".to_string())));
        let spec = Specificity::calculate(&sel);
        assert_eq!(spec.id_count, 1);
    }
}
