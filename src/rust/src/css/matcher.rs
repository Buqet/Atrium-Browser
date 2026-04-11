




use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use rustc_hash::FxHashMap;
use rayon::prelude::*;

use crate::html::HtmlNode;
use crate::css::selector::{Selector, ElementState, Specificity};
use crate::css::parser::{CssRule, MediaRule, Stylesheet, Declaration};
use crate::css::value::{CssValue, ViewportContext};
use crate::css::properties::{is_inherited_property, inherit_properties, CustomProperties};






#[derive(Clone, Debug)]
pub struct SelectorMatchCache {
    
    capacity: usize,
    
    entries: FxHashMap<u64, (bool, u32)>,
    
    order: VecDeque<u64>,
}

impl SelectorMatchCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: FxHashMap::default(),
            order: VecDeque::with_capacity(capacity),
        }
    }

    
    pub fn get(&mut self, key: u64) -> Option<bool> {
        if let Some(&mut (result, _)) = self.entries.get_mut(&key) {
            
            if let Some(pos) = self.order.iter().position(|&k| k == key) {
                self.order.remove(pos);
                self.order.push_back(key);
            }
            Some(result)
        } else {
            None
        }
    }

    
    pub fn insert(&mut self, key: u64, result: bool) {
        if self.entries.len() >= self.capacity {
            
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            }
        }
        self.entries.insert(key, (result, 0));
        self.order.push_back(key);
    }

    
    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    
    pub fn stats(&self) -> (usize, usize) {
        (self.entries.len(), self.capacity)
    }
}


fn compute_node_signature(node: &HtmlNode) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();

    if let HtmlNode::Element { tag, attributes, .. } = node {
        tag.hash(&mut hasher);
        
        
        if let Some(classes) = attributes.get("class") {
            classes.hash(&mut hasher);
        }
        
        
        if let Some(id) = attributes.get("id") {
            id.hash(&mut hasher);
        }
    } else {
        
        0u8.hash(&mut hasher);
    }

    hasher.finish()
}


fn compute_selector_key(selector: &Selector) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hash_selector(selector, &mut hasher);
    hasher.finish()
}


fn hash_selector(selector: &Selector, hasher: &mut impl Hasher) {
    match selector {
        Selector::Universal => 0u8.hash(hasher),
        Selector::Type(t) => { 1u8.hash(hasher); t.hash(hasher); }
        Selector::Class(c) => { 2u8.hash(hasher); c.hash(hasher); }
        Selector::Id(i) => { 3u8.hash(hasher); i.hash(hasher); }
        Selector::Attribute(n, v) => {
            4u8.hash(hasher);
            n.hash(hasher);
            if let Some((val, mt)) = v {
                val.hash(hasher);
                std::mem::discriminant(mt).hash(hasher);
            }
        }
        Selector::Descendant(a, b) => {
            5u8.hash(hasher);
            hash_selector(a, hasher);
            hash_selector(b, hasher);
        }
        Selector::Child(a, b) => {
            6u8.hash(hasher);
            hash_selector(a, hasher);
            hash_selector(b, hasher);
        }
        Selector::Adjacent(a, b) => {
            7u8.hash(hasher);
            hash_selector(a, hasher);
            hash_selector(b, hasher);
        }
        Selector::GeneralSibling(a, b) => {
            8u8.hash(hasher);
            hash_selector(a, hasher);
            hash_selector(b, hasher);
        }
        Selector::PseudoClass(n) => { 9u8.hash(hasher); n.hash(hasher); }
        Selector::PseudoElement(n) => { 10u8.hash(hasher); n.hash(hasher); }
        Selector::Group(sels) => {
            11u8.hash(hasher);
            for s in sels {
                hash_selector(s, hasher);
            }
        }
        Selector::Not(s) => {
            12u8.hash(hasher);
            hash_selector(s, hasher);
        }
        Selector::NthChild(a, b) => {
            13u8.hash(hasher);
            a.hash(hasher);
            b.hash(hasher);
        }
    }
}






#[derive(Clone, Debug, Default)]
pub struct RuleIndex {
    
    by_tag: FxHashMap<String, Vec<IndexedRule>>,
    
    by_class: FxHashMap<String, Vec<IndexedRule>>,
    
    by_id: FxHashMap<String, Vec<IndexedRule>>,
    
    universal: Vec<IndexedRule>,
}


#[derive(Clone, Debug)]
pub struct IndexedRule {
    pub rule_index: usize,
    pub specificity: Specificity,
    pub selector_index: usize,
}

impl RuleIndex {
    
    pub fn build(rules: &[CssRule]) -> Self {
        let mut index = Self::default();

        for (rule_idx, rule) in rules.iter().enumerate() {
            for (sel_idx, selector) in rule.selectors.iter().enumerate() {
                let specificity = Specificity::calculate(selector);
                let indexed = IndexedRule {
                    rule_index: rule_idx,
                    specificity,
                    selector_index: sel_idx,
                };

                Self::index_selector(&mut index, selector, indexed);
            }
        }

        index
    }

    fn index_selector(index: &mut Self, selector: &Selector, indexed: IndexedRule) {
        match selector {
            Selector::Type(tag) => {
                index.by_tag.entry(tag.to_lowercase()).or_default().push(indexed);
            }
            Selector::Class(class) => {
                index.by_class.entry(class.to_string()).or_default().push(indexed);
            }
            Selector::Id(id) => {
                index.by_id.entry(id.to_string()).or_default().push(indexed);
            }
            Selector::Universal | Selector::PseudoClass(_) => {
                index.universal.push(indexed);
            }
            
            Selector::Descendant(_, right)
            | Selector::Child(_, right)
            | Selector::Adjacent(_, right)
            | Selector::GeneralSibling(_, right) => {
                Self::index_selector(index, right, indexed);
            }
            Selector::Group(selectors) => {
                for s in selectors {
                    Self::index_selector(index, s, indexed.clone());
                }
            }
            Selector::Not(inner) => {
                Self::index_selector(index, inner, indexed);
            }
            Selector::NthChild(_, _) => {
                index.universal.push(indexed);
            }
            Selector::PseudoElement(_) => {} 
            Selector::Attribute(name, _) => {
                index.by_class.entry(format!("[{}]", name)).or_default().push(indexed);
            }
        }
    }

    
    pub fn get_candidates(
        &self,
        tag: Option<&str>,
        classes: &[&str],
        id: Option<&str>,
    ) -> Vec<&IndexedRule> {
        let mut candidates = Vec::new();

        
        candidates.extend(self.universal.iter());

        
        if let Some(t) = tag {
            if let Some(rules) = self.by_tag.get(&t.to_lowercase()) {
                candidates.extend(rules.iter());
            }
        }

        
        for class in classes {
            if let Some(rules) = self.by_class.get(*class) {
                candidates.extend(rules.iter());
            }
        }

        
        if let Some(i) = id {
            if let Some(rules) = self.by_id.get(i) {
                candidates.extend(rules.iter());
            }
        }

        candidates
    }
}








pub fn compute_styles(
    stylesheet: &Stylesheet,
    nodes: &[HtmlNode],
    states: &HashMap<usize, ElementState>,
    viewport_width: f32,
    viewport_height: f32,
) -> Vec<FxHashMap<String, CssValue>> {
    
    let applicable_rules = get_applicable_rules(stylesheet, viewport_width, viewport_height);

    
    let rule_index = RuleIndex::build(&applicable_rules);

    
    let mut custom_props = CustomProperties::new();
    custom_props.extract_from_rules(&applicable_rules);

    
    let rule_index = Arc::new(rule_index);
    let applicable_rules = Arc::new(applicable_rules);
    let states = Arc::new(states.clone());
    let custom_props = Arc::new(custom_props);

    
    let total_nodes = count_nodes(nodes);

    
    let cache_capacity = std::cmp::min(total_nodes * 50, 100_000);
    let cache = Arc::new(std::sync::Mutex::new(SelectorMatchCache::new(cache_capacity)));

    

    
    let mut root_ranges: Vec<(usize, usize)> = Vec::new(); 
    let mut offset = 0usize;
    for node in nodes {
        let count = count_node_descendants(node) + 1;
        root_ranges.push((offset, count));
        offset += count;
    }

    
    let styles: Vec<std::sync::Mutex<Option<FxHashMap<String, CssValue>>>> =
        (0..total_nodes).map(|_| std::sync::Mutex::new(None)).collect();
    let styles_ref = &styles;

    
    nodes.par_iter().enumerate().for_each(|(i, node)| {
        let (range_start, _range_count) = root_ranges[i];

        compute_node_styles_parallel(
            node,
            &rule_index,
            &applicable_rules,
            None,
            None,
            styles_ref,
            range_start,
            i,
            &states,
            &custom_props,
            16.0,
            16.0,
            viewport_width,
            viewport_height,
            Some(nodes),
            i,
            &cache,
        );
    });

    
    styles.into_iter().map(|s| s.into_inner().unwrap().unwrap_or_default()).collect()
}


fn count_nodes(nodes: &[HtmlNode]) -> usize {
    nodes.iter().map(|n| count_node_descendants(n) + 1).sum()
}


fn count_node_descendants(node: &HtmlNode) -> usize {
    if let HtmlNode::Element { children, .. } = node {
        children.iter().map(|c| count_node_descendants(c) + 1).sum()
    } else {
        0
    }
}






fn compute_node_styles_parallel(
    node: &HtmlNode,
    rule_index: &Arc<RuleIndex>,
    applicable_rules: &Arc<Vec<CssRule>>,
    parent: Option<&HtmlNode>,
    parent_style: Option<&FxHashMap<String, CssValue>>,
    styles: &[std::sync::Mutex<Option<FxHashMap<String, CssValue>>>],
    base_index: usize,
    base_node_index: usize,
    states: &Arc<HashMap<usize, ElementState>>,
    custom_props: &Arc<CustomProperties>,
    parent_font_size: f32,
    root_font_size: f32,
    viewport_width: f32,
    viewport_height: f32,
    siblings: Option<&[HtmlNode]>,
    sibling_index: usize,
    cache: &Arc<std::sync::Mutex<SelectorMatchCache>>,
) {
    
    let node_index = base_node_index;
    let state = states.get(&node_index).cloned().unwrap_or_default();

    let mut node_styles: FxHashMap<String, CssValue> = FxHashMap::default();
    let mut current_font_size = root_font_size;

    let (tag, classes, id) = match node {
        HtmlNode::Element { tag, attributes, .. } => {
            let tag = Some(tag.as_str());
            let classes: Vec<&str> = attributes.get("class")
                .map(|c| c.split_whitespace().collect::<Vec<_>>())
                .unwrap_or_default();
            let id = attributes.get("id").map(|s| s.as_str());
            (tag, classes, id)
        }
        _ => (None, vec![], None),
    };

    let candidates = rule_index.get_candidates(tag, &classes, id);

    
    let node_signature = compute_node_signature(node);

    
    let mut matched_declarations: Vec<(Specificity, usize, bool, String, CssValue)> =
        if candidates.len() > 100 {
            candidates.par_iter().flat_map(|indexed| {
                let rule = &applicable_rules[indexed.rule_index];
                let selector = &rule.selectors[indexed.selector_index];
                let selector_key = compute_selector_key(selector);
                let cache_key = selector_key ^ node_signature;

                
                let cached = {
                    let mut c = cache.lock().unwrap();
                    c.get(cache_key)
                };

                let matches = if let Some(result) = cached {
                    result
                } else {
                    let result = matches_selector_simple(selector, node, parent, &state, siblings, sibling_index);
                    {
                        let mut c = cache.lock().unwrap();
                        c.insert(cache_key, result);
                    }
                    result
                };

                if matches {
                    rule.declarations.iter().map(|decl| (
                        indexed.specificity.clone(),
                        indexed.rule_index,
                        decl.important,
                        decl.property.clone(),
                        decl.value.clone(),
                    )).collect::<Vec<_>>()
                } else {
                    Vec::new()
                }
            }).collect()
        } else {
            
            candidates.iter().flat_map(|indexed| {
                let rule = &applicable_rules[indexed.rule_index];
                let selector = &rule.selectors[indexed.selector_index];

                if matches_selector_simple(selector, node, parent, &state, siblings, sibling_index) {
                    rule.declarations.iter().map(|decl| (
                        indexed.specificity.clone(),
                        indexed.rule_index,
                        decl.important,
                        decl.property.clone(),
                        decl.value.clone(),
                    )).collect::<Vec<_>>()
                } else {
                    Vec::new()
                }
            }).collect()
        };

    matched_declarations.sort_by(|a, b| {
        if a.2 != b.2 {
            return a.2.cmp(&b.2);
        }
        let spec_cmp = a.0.cmp(&b.0);
        if spec_cmp != std::cmp::Ordering::Equal {
            return spec_cmp;
        }
        a.1.cmp(&b.1)
    });

    for (_, _, _, property, value) in matched_declarations {
        if property.starts_with("--") {
            node_styles.insert(property, value);
        } else {
            let resolved = resolve_css_value(&value, custom_props);
            node_styles.insert(property, resolved);
        }
    }

    if let Some(font_size_val) = node_styles.get("font-size") {
        if let CssValue::Number(n) = font_size_val {
            current_font_size = *n;
        } else if let CssValue::Length(len) = font_size_val {
            let ctx = ViewportContext {
                viewport_width,
                viewport_height,
                font_size: parent_font_size,
                root_font_size,
                containing_block_px: None,
            };
            current_font_size = len.to_px(&ctx);
        }
    }

    if let Some(ps) = parent_style {
        inherit_properties_map(&mut node_styles, ps);
    }

    
    if base_index < styles.len() {
        let mut guard = styles[base_index].lock().unwrap();
        *guard = Some(node_styles.clone());
    }

    
    if let HtmlNode::Element { children, .. } = node {
        if !children.is_empty() {
            
            let child_sizes: Vec<usize> = children.iter()
                .map(|c| count_node_descendants(c) + 1)
                .collect();

            
            let child_base_indices: Vec<usize> = {
                let mut indices = Vec::with_capacity(child_sizes.len());
                let mut running = base_index + 1;
                for size in &child_sizes {
                    indices.push(running);
                    running += size;
                }
                indices
            };

            
            children.par_iter().enumerate().for_each(|(child_idx, child)| {
                compute_node_styles_parallel(
                    child,
                    rule_index,
                    applicable_rules,
                    Some(node),
                    Some(&node_styles),
                    styles,
                    child_base_indices[child_idx],
                    child_base_indices[child_idx],
                    states,
                    custom_props,
                    current_font_size,
                    root_font_size,
                    viewport_width,
                    viewport_height,
                    Some(children),
                    child_idx,
                    cache,
                );
            });
        }
    }
}



fn matches_selector_simple(
    selector: &Selector,
    node: &HtmlNode,
    parent: Option<&HtmlNode>,
    state: &ElementState,
    siblings: Option<&[HtmlNode]>,
    sibling_index: usize,
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
            matches_attribute_selector(node, name, value.as_ref())
        }
        Selector::PseudoClass(name) => {
            match_pseudo_class_full(name, node, parent, siblings, sibling_index, state)
        }
        Selector::PseudoElement(_) => false, 
        Selector::Descendant(ancestor_sel, descendant_sel) => {
            
            if !matches_selector_simple(descendant_sel, node, parent, state, siblings, sibling_index) {
                return false;
            }
            
            find_ancestor_matching(node, parent, |ancestor| {
                matches_selector_simple(ancestor_sel, ancestor, 
                    get_parent_of(ancestor, parent), state,
                    get_siblings_of(ancestor, siblings, sibling_index),
                    get_sibling_index_of(ancestor, siblings, sibling_index))
            })
        }
        Selector::Child(parent_sel, child_sel) => {
            
            if !matches_selector_simple(child_sel, node, parent, state, siblings, sibling_index) {
                return false;
            }
            
            parent.map(|p| {
                matches_selector_simple(parent_sel, p, 
                    get_parent_of(p, parent), state,
                    get_siblings_of(p, siblings, sibling_index),
                    get_sibling_index_of(p, siblings, sibling_index))
            }).unwrap_or(false)
        }
        Selector::Adjacent(prev_sel, next_sel) => {
            
            if !matches_selector_simple(next_sel, node, parent, state, siblings, sibling_index) {
                return false;
            }
            
            if let Some(sibs) = siblings {
                if sibling_index > 0 {
                    let prev = &sibs[sibling_index - 1];
                    matches_selector_simple(prev_sel, prev, parent, state,
                        Some(sibs), sibling_index - 1)
                } else {
                    false
                }
            } else {
                false
            }
        }
        Selector::GeneralSibling(sibling_sel, current_sel) => {
            
            if !matches_selector_simple(current_sel, node, parent, state, siblings, sibling_index) {
                return false;
            }
            
            if let Some(sibs) = siblings {
                for i in 0..sibling_index {
                    if matches_selector_simple(sibling_sel, &sibs[i], parent, state,
                        Some(sibs), i) {
                        return true;
                    }
                }
            }
            false
        }
        Selector::Group(selectors) => {
            selectors.iter().any(|s| matches_selector_simple(s, node, parent, state, siblings, sibling_index))
        }
        Selector::Not(inner) => {
            !matches_selector_simple(inner, node, parent, state, siblings, sibling_index)
        }
        Selector::NthChild(a, b) => {
            if let Some(sibs) = siblings {
                let n = sibling_index as i32 + 1; 
                if *a == 0 {
                    n == *b
                } else if *a > 0 {
                    (n - *b) % *a == 0 && (n - *b) / *a >= 0
                } else {
                    
                    (n - *b) % *a == 0 && (n - *b) / *a <= 0
                }
            } else {
                false
            }
        }
    }
}


fn matches_attribute_selector(
    node: &HtmlNode,
    name: &str,
    value: Option<&(String, AttributeMatchType)>,
) -> bool {
    if let HtmlNode::Element { attributes, .. } = node {
        match value {
            None => attributes.contains_key(name),
            Some((val, match_type)) => {
                if let Some(attr_val) = attributes.get(name) {
                    match match_type {
                        AttributeMatchType::Exact => attr_val == val,
                        AttributeMatchType::Includes => {
                            
                            attr_val.split_whitespace().any(|w| w == val)
                        }
                        AttributeMatchType::DashMatch => {
                            
                            attr_val == val || attr_val.starts_with(&format!("{}-", val))
                        }
                        AttributeMatchType::Prefix => {
                            
                            attr_val.starts_with(val)
                        }
                        AttributeMatchType::Suffix => {
                            
                            attr_val.ends_with(val)
                        }
                        AttributeMatchType::Substring => {
                            
                            attr_val.contains(val)
                        }
                    }
                } else {
                    false
                }
            }
        }
    } else {
        false
    }
}


#[derive(Clone, Debug)]
pub enum AttributeMatchType {
    Exact,      
    Includes,   
    DashMatch,  
    Prefix,     
    Suffix,     
    Substring,  
}


fn match_pseudo_class_full(
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
        
        
        "link" => true, 
        "visited" => false, 
        
        
        "checked" => state.checked,
        "disabled" => state.disabled,
        "enabled" => !state.disabled,
        "indeterminate" => false, 
        "default" => sibling_index == 0, 
        "valid" => true, 
        "invalid" => false,
        "in-range" => true,
        "out-of-range" => false,
        "required" => false, 
        "optional" => true,
        "read-only" => false,
        "read-write" => true,
        
        
        "first-child" => sibling_index == 0,
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
        "only-of-type" => {
            if let Some(sibs) = siblings {
                if let HtmlNode::Element { tag, .. } = node {
                    let count = sibs.iter().filter(|s| {
                        if let HtmlNode::Element { tag: s_tag, .. } = s {
                            s_tag == tag
                        } else {
                            false
                        }
                    }).count();
                    count == 1
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
        "target" => false, 
        _ => false,
    }
}


fn find_ancestor_matching<'a>(
    node: &'a HtmlNode,
    parent: Option<&'a HtmlNode>,
    mut matcher: impl FnMut(&'a HtmlNode) -> bool,
) -> bool {
    let mut current = parent;
    while let Some(p) = current {
        if matcher(p) {
            return true;
        }
        
        current = get_parent_of(p, parent);
    }
    false
}


fn get_parent_of<'a>(node: &'a HtmlNode, parent: Option<&'a HtmlNode>) -> Option<&'a HtmlNode> {
    
    
    parent
}


fn get_siblings_of<'a>(
    node: &'a HtmlNode,
    siblings: Option<&'a [HtmlNode]>,
    sibling_index: usize,
) -> Option<&'a [HtmlNode]> {
    siblings
}


fn get_sibling_index_of(
    node: &HtmlNode,
    siblings: Option<&[HtmlNode]>,
    sibling_index: usize,
) -> usize {
    sibling_index
}


fn resolve_css_value(value: &CssValue, custom_props: &CustomProperties) -> CssValue {
    match value {
        CssValue::String(s) => {
            let resolved = custom_props.resolve_var(s);
            parse_resolved_value(&resolved).unwrap_or(CssValue::String(std::borrow::Cow::Owned(resolved)))
        }
        CssValue::Keyword(k) => {
            let resolved = custom_props.resolve_var(k);
            parse_resolved_value(&resolved).unwrap_or(CssValue::Keyword(std::borrow::Cow::Owned(resolved)))
        }
        other => other.clone(),
    }
}


fn parse_resolved_value(s: &str) -> Option<CssValue> {
    let s = s.trim();

    if s.eq_ignore_ascii_case("none") {
        Some(CssValue::None)
    } else if s.eq_ignore_ascii_case("auto") {
        Some(CssValue::Auto)
    } else if s.eq_ignore_ascii_case("inherit") {
        Some(CssValue::Inherit)
    } else if s.eq_ignore_ascii_case("initial") {
        Some(CssValue::Initial)
    } else if s.eq_ignore_ascii_case("unset") {
        Some(CssValue::Unset)
    } else if s.eq_ignore_ascii_case("revert") {
        Some(CssValue::Revert)
    } else if s.starts_with('#') {
        crate::css::value::Color::from_hex(&s[1..]).map(CssValue::Color)
    } else if s.starts_with("url(") {
        let url = s.trim_start_matches("url(").trim_end_matches(')').trim_matches('"').trim_matches('\'');
        Some(CssValue::Url(std::borrow::Cow::Owned(url.to_string())))
    } else if let Ok(num) = s.parse::<f32>() {
        Some(CssValue::Number(num))
    } else {
        Some(CssValue::Keyword(std::borrow::Cow::Owned(s.to_string())))
    }
}







fn inherit_properties_map(
    node_styles: &mut FxHashMap<String, CssValue>,
    parent_styles: &FxHashMap<String, CssValue>,
) {
    
    let keys_to_process: Vec<String> = node_styles.keys().cloned().collect();
    
    for property in keys_to_process {
        if let Some(value) = node_styles.get(&property) {
            match value {
                CssValue::Inherit => {
                    
                    if let Some(parent_val) = parent_styles.get(&property) {
                        node_styles.insert(property.clone(), parent_val.clone());
                    }
                }
                CssValue::Initial => {
                    
                    node_styles.remove(&property);
                }
                CssValue::Unset => {
                    if is_inherited_property(&property) {
                        
                        if let Some(parent_val) = parent_styles.get(&property) {
                            node_styles.insert(property.clone(), parent_val.clone());
                        }
                    } else {
                        
                        node_styles.remove(&property);
                    }
                }
                CssValue::Revert => {
                    
                    node_styles.remove(&property);
                }
                _ => {}
            }
        }
    }
    
    
    for (property, value) in parent_styles {
        if is_inherited_property(property) && !node_styles.contains_key(property) {
            
            if !matches!(value, CssValue::Initial | CssValue::Unset | CssValue::Revert) {
                node_styles.insert(property.clone(), value.clone());
            }
        }
    }
}



pub fn evaluate_media_query(media_rule: &MediaRule, viewport_width: f32, viewport_height: f32) -> bool {
    if media_rule.conditions.is_empty() {
        return true;
    }

    for condition in &media_rule.conditions {
        let value_px = match condition.unit.as_str() {
            "px" => condition.value,
            "em" | "rem" => condition.value * 16.0,
            _ => condition.value,
        };

        match condition.feature.as_str() {
            
            "max-width" => {
                if viewport_width > value_px { return false; }
            }
            "min-width" => {
                if viewport_width < value_px { return false; }
            }
            "max-height" => {
                if viewport_height > value_px { return false; }
            }
            "min-height" => {
                if viewport_height < value_px { return false; }
            }
            
            
            "aspect-ratio" | "min-aspect-ratio" | "max-aspect-ratio" => {
                let actual_ratio = viewport_width / viewport_height;
                let cond_ratio = value_px;
                match condition.feature.as_str() {
                    "aspect-ratio" => if (actual_ratio - cond_ratio).abs() > 0.01 { return false; },
                    "min-aspect-ratio" => if actual_ratio < cond_ratio { return false; },
                    "max-aspect-ratio" => if actual_ratio > cond_ratio { return false; },
                    _ => {}
                }
            }
            "orientation" => {
                
                let is_landscape = viewport_width >= viewport_height;
                let cond_landscape = condition.value >= 1.0; 
                if is_landscape != cond_landscape { return false; }
            }
            
            
            "resolution" | "min-resolution" | "max-resolution" => {
                
                let actual_dppx = 1.0; 
                match condition.feature.as_str() {
                    "resolution" => if (actual_dppx - value_px).abs() > 0.01 { return false; },
                    "min-resolution" => if actual_dppx < value_px { return false; },
                    "max-resolution" => if actual_dppx > value_px { return false; },
                    _ => {}
                }
            }
            
            
            "grid" => {
                
                
                if condition.value > 0.0 { return false; }
            }
            "update" | "scan" => {
                
            }
            
            
            "color" | "min-color" | "max-color" => {
                
                let actual_color = 8.0;
                match condition.feature.as_str() {
                    "color" => if (actual_color - value_px).abs() > 0.01 { return false; },
                    "min-color" => if actual_color < value_px { return false; },
                    "max-color" => if actual_color > value_px { return false; },
                    _ => {}
                }
            }
            "color-index" | "min-color-index" | "max-color-index" => {
                
                let actual_index = 16777216.0;
                match condition.feature.as_str() {
                    "color-index" => if (actual_index - value_px).abs() > 1000.0 { return false; },
                    "min-color-index" => if actual_index < value_px { return false; },
                    "max-color-index" => if actual_index > value_px { return false; },
                    _ => {}
                }
            }
            "monochrome" | "min-monochrome" | "max-monochrome" => {
                
                let actual_mono = 0.0;
                match condition.feature.as_str() {
                    "monochrome" => if (actual_mono - value_px).abs() > 0.01 { return false; },
                    "min-monochrome" => if actual_mono < value_px { return false; },
                    "max-monochrome" => if actual_mono > value_px { return false; },
                    _ => {}
                }
            }
            
            
            "prefers-color-scheme" => {
                
                
                if condition.value > 0.5 { return false; }
            }
            "prefers-reduced-motion" => {
                
                
                if condition.value > 0.5 { return false; }
            }
            "prefers-contrast" => {
                
                
                if condition.value > 0.5 { return false; }
            }
            
            
            "pointer" => {
                
                
                if condition.value < 1.5 { return false; }
            }
            "hover" => {
                
                
                if condition.value < 0.5 { return false; }
            }
            "any-pointer" | "any-hover" => {
                
            }
            
            
            "overflow-block" | "overflow-inline" => {
                
            }
            
            _ => {
                
                return false;
            }
        }
    }

    true
}


pub fn get_applicable_rules(
    stylesheet: &Stylesheet,
    viewport_width: f32,
    viewport_height: f32,
) -> Vec<CssRule> {
    let mut rules = stylesheet.rules.clone();

    for media_rule in &stylesheet.media_rules {
        if evaluate_media_query(media_rule, viewport_width, viewport_height) {
            rules.extend(media_rule.rules.clone());
        }
    }

    rules
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_index_by_tag() {
        let mut parser = crate::css::parser::CssParser::new();
        let stylesheet = parser.parse("div { color: red; } p { color: blue; }").unwrap();

        let index = RuleIndex::build(&stylesheet.rules);
        assert!(index.by_tag.contains_key("div"));
        assert!(index.by_tag.contains_key("p"));
    }

    #[test]
    fn test_rule_index_by_class() {
        let mut parser = crate::css::parser::CssParser::new();
        let stylesheet = parser.parse(".foo { color: red; }").unwrap();

        let index = RuleIndex::build(&stylesheet.rules);
        assert!(index.by_class.contains_key("foo"));
    }

    #[test]
    fn test_media_query_evaluation() {
        let media_rule = MediaRule {
            query: "(max-width: 600px)".to_string(),
            conditions: vec![
                crate::css::parser::MediaQuery {
                    feature: "max-width".to_string(),
                    value: 600.0,
                    unit: "px".to_string(),
                }
            ],
            rules: vec![],
        };

        assert!(evaluate_media_query(&media_rule, 500.0, 800.0));
        assert!(!evaluate_media_query(&media_rule, 800.0, 800.0));
    }
}
