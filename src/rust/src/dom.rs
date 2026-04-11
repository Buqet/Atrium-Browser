








use std::collections::HashMap;
use rayon::prelude::*;

use crate::html::HtmlNode;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeHandle(pub u64);

impl NodeHandle {
    pub fn new(id: u64) -> Self {
        NodeHandle(id)
    }
}


#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    
    Document,
    
    DocumentFragment,
    
    Element {
        tag_name: String,
        attributes: HashMap<String, String>,
    },
    
    Text(String),
    
    Comment(String),
    
    Doctype {
        name: String,
        public_id: Option<String>,
        system_id: Option<String>,
    },
}


#[derive(Debug, Clone)]
pub struct Node {
    pub handle: NodeHandle,
    pub node_type: NodeType,
    pub parent: Option<NodeHandle>,
    pub children: Vec<NodeHandle>,
    pub next_sibling: Option<NodeHandle>,
    pub previous_sibling: Option<NodeHandle>,
}

impl Node {
    pub fn new(handle: NodeHandle, node_type: NodeType) -> Self {
        Node {
            handle,
            node_type,
            parent: None,
            children: Vec::new(),
            next_sibling: None,
            previous_sibling: None,
        }
    }

    
    pub fn tag_name(&self) -> Option<&str> {
        match &self.node_type {
            NodeType::Element { tag_name, .. } => Some(tag_name),
            _ => None,
        }
    }

    
    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        match &self.node_type {
            NodeType::Element { attributes, .. } => attributes.get(name).map(|s| s.as_str()),
            _ => None,
        }
    }

    
    pub fn set_attribute(&mut self, name: String, value: String) -> Option<String> {
        match &mut self.node_type {
            NodeType::Element { attributes, .. } => attributes.insert(name, value),
            _ => None,
        }
    }

    
    pub fn remove_attribute(&mut self, name: &str) -> Option<String> {
        match &mut self.node_type {
            NodeType::Element { attributes, .. } => attributes.remove(name),
            _ => None,
        }
    }

    
    pub fn has_attribute(&self, name: &str) -> bool {
        match &self.node_type {
            NodeType::Element { attributes, .. } => attributes.contains_key(name),
            _ => false,
        }
    }

    
    pub fn text_content(&self) -> Option<&str> {
        match &self.node_type {
            NodeType::Text(text) => Some(text),
            _ => None,
        }
    }
}


pub struct ChildrenIterator<'a> {
    document: &'a Document,
    child_iter: std::vec::IntoIter<NodeHandle>,
}

impl<'a> Iterator for ChildrenIterator<'a> {
    type Item = NodeHandle;

    fn next(&mut self) -> Option<Self::Item> {
        self.child_iter.next()
    }
}


pub struct DescendantsIterator<'a> {
    document: &'a Document,
    stack: Vec<std::vec::IntoIter<NodeHandle>>,
}

impl<'a> DescendantsIterator<'a> {
    fn new(document: &'a Document, root: NodeHandle) -> Self {
        let mut stack = Vec::new();
        if let Some(node) = document.nodes.get(&root) {
            stack.push(node.children.clone().into_iter());
        }
        Self { document, stack }
    }
}

impl<'a> Iterator for DescendantsIterator<'a> {
    type Item = NodeHandle;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let top = self.stack.last_mut()?;
            if let Some(child) = top.next() {
                
                if let Some(node) = self.document.nodes.get(&child) {
                    self.stack.push(node.children.clone().into_iter());
                }
                return Some(child);
            } else {
                self.stack.pop();
            }
        }
    }
}


#[derive(Debug, Clone)]
pub struct Document {
    pub root: NodeHandle,
    pub nodes: HashMap<NodeHandle, Node>,
    pub next_id: u64,
}

impl Document {
    pub fn new() -> Self {
        let root_handle = NodeHandle::new(0);
        let root_node = Node::new(root_handle, NodeType::Document);

        let mut nodes = HashMap::new();
        nodes.insert(root_handle, root_node);

        Document {
            root: root_handle,
            nodes,
            next_id: 1,
        }
    }

    
    pub fn allocate_handle(&mut self) -> NodeHandle {
        let handle = NodeHandle::new(self.next_id);
        self.next_id += 1;
        handle
    }

    
    pub fn create_element(&mut self, tag_name: &str) -> NodeHandle {
        let handle = self.allocate_handle();
        let node = Node::new(
            handle,
            NodeType::Element {
                tag_name: tag_name.to_string(),
                attributes: HashMap::new(),
            },
        );
        self.nodes.insert(handle, node);
        handle
    }

    
    pub fn create_text_node(&mut self, text: &str) -> NodeHandle {
        let handle = self.allocate_handle();
        let node = Node::new(handle, NodeType::Text(text.to_string()));
        self.nodes.insert(handle, node);
        handle
    }

    
    pub fn create_document_fragment(&mut self) -> NodeHandle {
        let handle = self.allocate_handle();
        let node = Node::new(handle, NodeType::DocumentFragment);
        self.nodes.insert(handle, node);
        handle
    }

    
    pub fn has_node(&self, handle: NodeHandle) -> bool {
        self.nodes.contains_key(&handle)
    }

    
    fn is_descendant_of(&self, descendant: NodeHandle, ancestor: NodeHandle) -> bool {
        let mut current = self.nodes.get(&descendant).and_then(|n| n.parent);
        while let Some(parent) = current {
            if parent == ancestor {
                return true;
            }
            current = self.nodes.get(&parent).and_then(|n| n.parent);
        }
        false
    }

    
    
    
    pub fn append_child(&mut self, parent: NodeHandle, child: NodeHandle) -> bool {
        
        if !self.nodes.contains_key(&parent) || !self.nodes.contains_key(&child) {
            return false;
        }

        
        if self.is_descendant_of(parent, child) {
            return false;
        }

        
        let old_parent = self.nodes.get(&child).and_then(|n| n.parent);
        let last_child = self.nodes.get(&parent)
            .and_then(|p| p.children.last().copied());

        
        if let Some(op) = old_parent {
            self.remove_child_internal(op, child);
        }

        
        if let Some(child_node) = self.nodes.get_mut(&child) {
            child_node.parent = Some(parent);
        }

        
        if let Some(last_child) = last_child {
            if let Some(last_child_node) = self.nodes.get_mut(&last_child) {
                last_child_node.next_sibling = Some(child);
            }
            if let Some(child_node) = self.nodes.get_mut(&child) {
                child_node.previous_sibling = Some(last_child);
            }
        }

        
        if let Some(parent_node) = self.nodes.get_mut(&parent) {
            parent_node.children.push(child);
        }
        true
    }

    
    
    
    pub fn remove_child(&mut self, parent: NodeHandle, child: NodeHandle) -> Option<NodeHandle> {
        if !self.nodes.contains_key(&parent) || !self.nodes.contains_key(&child) {
            return None;
        }

        
        let belongs = self.nodes.get(&child)
            .and_then(|n| n.parent)
            .map(|p| p == parent)
            .unwrap_or(false);

        if !belongs {
            return None;
        }

        self.remove_child_internal(parent, child);
        Some(child)
    }

    
    fn remove_child_internal(&mut self, parent: NodeHandle, child: NodeHandle) {
        
        let (prev, next) = if let Some(child_node) = self.nodes.get(&child) {
            (child_node.previous_sibling, child_node.next_sibling)
        } else {
            (None, None)
        };

        
        if let Some(parent_node) = self.nodes.get_mut(&parent) {
            parent_node.children.retain(|&c| c != child);
        }

        
        if let Some(prev_handle) = prev {
            if let Some(prev_node) = self.nodes.get_mut(&prev_handle) {
                prev_node.next_sibling = next;
            }
        }
        if let Some(next_handle) = next {
            if let Some(next_node) = self.nodes.get_mut(&next_handle) {
                next_node.previous_sibling = prev;
            }
        }

        
        if let Some(child_node) = self.nodes.get_mut(&child) {
            child_node.parent = None;
            child_node.previous_sibling = None;
            child_node.next_sibling = None;
        }
    }

    
    
    
    
    pub fn insert_before(&mut self, parent: NodeHandle, new_child: NodeHandle, ref_child: Option<NodeHandle>) -> bool {
        if !self.nodes.contains_key(&parent) || !self.nodes.contains_key(&new_child) {
            return false;
        }

        
        if self.is_descendant_of(parent, new_child) {
            return false;
        }

        
        if let Some(old_parent) = self.nodes.get(&new_child).and_then(|n| n.parent) {
            self.remove_child_internal(old_parent, new_child);
        }

        let ref_child = match ref_child {
            Some(rc) => rc,
            None => return self.append_child(parent, new_child),
        };

        
        let is_child = self.nodes.get(&parent)
            .map(|p| p.children.contains(&ref_child))
            .unwrap_or(false);
        if !is_child {
            return false;
        }

        
        self.nodes.get_mut(&new_child).unwrap().parent = Some(parent);

        
        if let Some(parent_node) = self.nodes.get_mut(&parent) {
            if let Some(idx) = parent_node.children.iter().position(|&c| c == ref_child) {
                parent_node.children.insert(idx, new_child);
            }
        }

        
        let prev_sibling = self.nodes.get(&ref_child)
            .and_then(|n| n.previous_sibling);

        self.nodes.get_mut(&new_child).unwrap().previous_sibling = prev_sibling;
        self.nodes.get_mut(&new_child).unwrap().next_sibling = Some(ref_child);

        if let Some(prev_handle) = prev_sibling {
            if let Some(prev_node) = self.nodes.get_mut(&prev_handle) {
                prev_node.next_sibling = Some(new_child);
            }
        }

        
        if let Some(ref_node) = self.nodes.get_mut(&ref_child) {
            ref_node.previous_sibling = Some(new_child);
        }

        true
    }

    
    
    
    pub fn replace_child(&mut self, parent: NodeHandle, new_child: NodeHandle, old_child: NodeHandle) -> Option<NodeHandle> {
        if !self.nodes.contains_key(&parent)
            || !self.nodes.contains_key(&new_child)
            || !self.nodes.contains_key(&old_child)
        {
            return None;
        }

        
        if self.is_descendant_of(parent, new_child) {
            return None;
        }

        
        let belongs = self.nodes.get(&old_child)
            .and_then(|n| n.parent)
            .map(|p| p == parent)
            .unwrap_or(false);
        if !belongs {
            return None;
        }

        
        let old_parent_of_new = self.nodes.get(&new_child).and_then(|n| n.parent);
        let prev = self.nodes.get(&old_child).and_then(|n| n.previous_sibling);
        let next = self.nodes.get(&old_child).and_then(|n| n.next_sibling);

        
        if let Some(op) = old_parent_of_new {
            self.remove_child_internal(op, new_child);
        }

        
        if let Some(parent_node) = self.nodes.get_mut(&parent) {
            if let Some(idx) = parent_node.children.iter().position(|&c| c == old_child) {
                parent_node.children[idx] = new_child;
            }
        }

        
        if let Some(new_node) = self.nodes.get_mut(&new_child) {
            new_node.parent = Some(parent);
            new_node.previous_sibling = prev;
            new_node.next_sibling = next;
        }

        
        if let Some(prev_handle) = prev {
            if let Some(prev_node) = self.nodes.get_mut(&prev_handle) {
                prev_node.next_sibling = Some(new_child);
            }
        }
        if let Some(next_handle) = next {
            if let Some(next_node) = self.nodes.get_mut(&next_handle) {
                next_node.previous_sibling = Some(new_child);
            }
        }

        
        if let Some(old_node) = self.nodes.get_mut(&old_child) {
            old_node.parent = None;
            old_node.previous_sibling = None;
            old_node.next_sibling = None;
        }

        Some(old_child)
    }

    
    pub fn destroy_node(&mut self, handle: NodeHandle) {
        if !self.nodes.contains_key(&handle) {
            return;
        }

        
        if let Some(parent) = self.nodes.get(&handle).and_then(|n| n.parent) {
            self.remove_child_internal(parent, handle);
        }

        
        let mut to_remove = Vec::new();
        {
            let mut stack = vec![handle];
            while let Some(current) = stack.pop() {
                if let Some(node) = self.nodes.get(&current) {
                    for &child in &node.children {
                        stack.push(child);
                    }
                    if current != handle {
                        to_remove.push(current);
                    }
                }
            }
        }

        for h in to_remove {
            self.nodes.remove(&h);
        }
        self.nodes.remove(&handle);
    }

    
    pub fn children(&self, parent: NodeHandle) -> ChildrenIterator<'_> {
        let child_iter = self.nodes.get(&parent)
            .map(|n| n.children.clone().into_iter())
            .unwrap_or_else(|| Vec::new().into_iter());
        ChildrenIterator {
            document: self,
            child_iter,
        }
    }

    
    pub fn descendants(&self, root: NodeHandle) -> DescendantsIterator<'_> {
        DescendantsIterator::new(self, root)
    }

    
    pub fn node_text_content(&self, handle: NodeHandle) -> String {
        let mut text = String::new();
        let mut stack = vec![handle];

        while let Some(current) = stack.pop() {
            if let Some(node) = self.nodes.get(&current) {
                match &node.node_type {
                    NodeType::Text(s) => text.push_str(s),
                    _ => {
                        
                        for child in node.children.iter().rev() {
                            stack.push(*child);
                        }
                    }
                }
            }
        }

        text
    }

    
    pub fn from_html(html_nodes: &[HtmlNode]) -> Self {
        let mut doc = Document::new();
        let html_handle = doc.create_element("html");
        doc.append_child(doc.root, html_handle);

        for node in html_nodes {
            Self::insert_html_node(&mut doc, html_handle, node);
        }

        doc
    }

    fn insert_html_node(doc: &mut Document, parent: NodeHandle, html_node: &HtmlNode) {
        match html_node {
            HtmlNode::Element { tag, attributes, children } => {
                let handle = doc.create_element(tag);
                if let Some(node) = doc.nodes.get_mut(&handle) {
                    if let NodeType::Element { attributes: attrs, .. } = &mut node.node_type {
                        for (k, v) in attributes {
                            attrs.insert(k.clone(), v.clone());
                        }
                    }
                }
                doc.append_child(parent, handle);
                for child in children {
                    Self::insert_html_node(doc, handle, child);
                }
            }
            HtmlNode::Text(text) => {
                let handle = doc.create_text_node(text);
                doc.append_child(parent, handle);
            }
            HtmlNode::Comment(text) => {
                let handle = doc.allocate_handle();
                let node = Node::new(handle, NodeType::Comment(text.clone()));
                doc.nodes.insert(handle, node);
                doc.append_child(parent, handle);
            }
            HtmlNode::Doctype { name, public_id, system_id } => {
                let handle = doc.allocate_handle();
                let node = Node::new(handle, NodeType::Doctype {
                    name: name.clone(),
                    public_id: public_id.clone(),
                    system_id: system_id.clone(),
                });
                doc.nodes.insert(handle, node);
                doc.append_child(parent, handle);
            }
        }
    }

    
    pub fn get_node(&self, handle: NodeHandle) -> Option<&Node> {
        self.nodes.get(&handle)
    }

    
    pub fn get_node_mut(&mut self, handle: NodeHandle) -> Option<&mut Node> {
        self.nodes.get_mut(&handle)
    }

    
    pub fn query_selector(&self, selector: &str) -> Option<NodeHandle> {
        self.nodes.par_iter()
            .find_map_first(|(_, node)| {
                if let NodeType::Element { tag_name, .. } = &node.node_type {
                    if tag_name == selector {
                        return Some(node.handle);
                    }
                }
                None
            })
    }

    
    pub fn get_elements_by_tag_name(&self, tag_name: &str) -> Vec<NodeHandle> {
        self.nodes
            .par_iter()
            .filter_map(|(_, node)| {
                if let NodeType::Element { tag_name: tn, .. } = &node.node_type {
                    if tn == tag_name {
                        return Some(node.handle);
                    }
                }
                None
            })
            .collect()
    }

    
    pub fn document_element(&self) -> Option<NodeHandle> {
        let root = self.nodes.get(&self.root)?;
        root.children.first().copied()
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_document() {
        let doc = Document::new();
        assert_eq!(doc.next_id, 1);
        assert!(doc.nodes.contains_key(&doc.root));
    }

    #[test]
    fn test_create_element() {
        let mut doc = Document::new();
        let div = doc.create_element("div");
        let node = doc.get_node(div).unwrap();
        assert_eq!(node.tag_name(), Some("div"));
    }

    #[test]
    fn test_append_child() {
        let mut doc = Document::new();
        let html = doc.create_element("html");
        let body = doc.create_element("body");
        doc.append_child(html, body);

        let html_node = doc.get_node(html).unwrap();
        assert_eq!(html_node.children.len(), 1);

        let body_node = doc.get_node(body).unwrap();
        assert_eq!(body_node.parent, Some(html));
    }

    #[test]
    fn test_query_selector() {
        let mut doc = Document::new();
        let div = doc.create_element("div");
        let span = doc.create_element("span");
        doc.append_child(div, span);

        let result = doc.query_selector("span");
        assert!(result.is_some());
    }

    

    #[test]
    fn test_remove_child() {
        let mut doc = Document::new();
        let parent = doc.create_element("div");
        let child = doc.create_element("span");
        doc.append_child(parent, child);

        let removed = doc.remove_child(parent, child);
        assert_eq!(removed, Some(child));

        let child_node = doc.get_node(child).unwrap();
        assert!(child_node.parent.is_none());

        let parent_node = doc.get_node(parent).unwrap();
        assert!(parent_node.children.is_empty());
    }

    #[test]
    fn test_remove_child_updates_siblings() {
        let mut doc = Document::new();
        let parent = doc.create_element("div");
        let a = doc.create_element("span");
        let b = doc.create_element("span");
        let c = doc.create_element("span");
        doc.append_child(parent, a);
        doc.append_child(parent, b);
        doc.append_child(parent, c);

        
        doc.remove_child(parent, b);

        let a_node = doc.get_node(a).unwrap();
        assert_eq!(a_node.next_sibling, Some(c));

        let c_node = doc.get_node(c).unwrap();
        assert_eq!(c_node.previous_sibling, Some(a));
    }

    #[test]
    fn test_insert_before() {
        let mut doc = Document::new();
        let parent = doc.create_element("div");
        let existing = doc.create_element("span");
        doc.append_child(parent, existing);

        let new_child = doc.create_element("p");
        let result = doc.insert_before(parent, new_child, Some(existing));
        assert!(result);

        let parent_node = doc.get_node(parent).unwrap();
        assert_eq!(parent_node.children, vec![new_child, existing]);
    }

    #[test]
    fn test_insert_before_at_end() {
        let mut doc = Document::new();
        let parent = doc.create_element("div");
        let existing = doc.create_element("span");
        doc.append_child(parent, existing);

        let new_child = doc.create_element("p");
        let result = doc.insert_before(parent, new_child, None);
        assert!(result);

        let parent_node = doc.get_node(parent).unwrap();
        assert_eq!(parent_node.children, vec![existing, new_child]);
    }

    #[test]
    fn test_replace_child() {
        let mut doc = Document::new();
        let parent = doc.create_element("div");
        let old = doc.create_element("span");
        doc.append_child(parent, old);

        let new_child = doc.create_element("p");
        let result = doc.replace_child(parent, new_child, old);
        assert_eq!(result, Some(old));

        let parent_node = doc.get_node(parent).unwrap();
        assert_eq!(parent_node.children, vec![new_child]);

        let old_node = doc.get_node(old).unwrap();
        assert!(old_node.parent.is_none());
    }

    #[test]
    fn test_destroy_node() {
        let mut doc = Document::new();
        let parent = doc.create_element("div");
        let child = doc.create_element("span");
        doc.append_child(parent, child);

        doc.destroy_node(child);
        assert!(!doc.has_node(child));
    }

    #[test]
    fn test_children_iterator() {
        let mut doc = Document::new();
        let parent = doc.create_element("div");
        let a = doc.create_element("span");
        let b = doc.create_element("p");
        doc.append_child(parent, a);
        doc.append_child(parent, b);

        let children: Vec<_> = doc.children(parent).collect();
        assert_eq!(children, vec![a, b]);
    }

    #[test]
    fn test_descendants_iterator() {
        let mut doc = Document::new();
        let root = doc.create_element("div");
        let child1 = doc.create_element("span");
        let child2 = doc.create_element("p");
        let grandchild = doc.create_element("a");
        doc.append_child(root, child1);
        doc.append_child(root, child2);
        doc.append_child(child1, grandchild);

        let descendants: Vec<_> = doc.descendants(root).collect();
        assert_eq!(descendants, vec![child1, grandchild, child2]);
    }

    #[test]
    fn test_node_text_content() {
        let mut doc = Document::new();
        let div = doc.create_element("div");
        let p1 = doc.create_element("p");
        let t1 = doc.create_text_node("Hello ");
        let p2 = doc.create_element("p");
        let t2 = doc.create_text_node("World");
        doc.append_child(div, p1);
        doc.append_child(p1, t1);
        doc.append_child(div, p2);
        doc.append_child(p2, t2);

        let text = doc.node_text_content(div);
        assert_eq!(text, "Hello World");
    }

    #[test]
    fn test_remove_attribute() {
        let mut doc = Document::new();
        let el = doc.create_element("div");
        let node = doc.get_node_mut(el).unwrap();
        node.set_attribute("id".to_string(), "main".to_string());
        assert!(node.has_attribute("id"));
        node.remove_attribute("id");
        assert!(!node.has_attribute("id"));
    }

    #[test]
    fn test_cycle_prevention() {
        let mut doc = Document::new();
        let a = doc.create_element("div");
        let b = doc.create_element("span");
        doc.append_child(a, b);

        
        let result = doc.append_child(b, a);
        assert!(!result);
    }

    #[test]
    fn test_invalid_handles() {
        let mut doc = Document::new();
        let invalid = NodeHandle::new(999);

        assert!(!doc.append_child(invalid, invalid));
        assert!(doc.remove_child(invalid, invalid).is_none());
        assert!(!doc.insert_before(invalid, invalid, None));
        assert!(doc.replace_child(invalid, invalid, invalid).is_none());
    }

    #[test]
    fn test_document_fragment() {
        let mut doc = Document::new();
        let frag = doc.create_document_fragment();
        let a = doc.create_element("span");
        let b = doc.create_element("span");
        doc.append_child(frag, a);
        doc.append_child(frag, b);

        let frag_node = doc.get_node(frag).unwrap();
        assert_eq!(frag_node.children.len(), 2);
        assert!(matches!(frag_node.node_type, NodeType::DocumentFragment));
    }

    #[test]
    fn test_from_html() {
        use crate::html::HtmlParser;
        let mut parser = HtmlParser::new();
        let html_nodes = parser.parse("<html><body><p>Hello</p></body></html>").unwrap();
        let doc = Document::from_html(&html_nodes);

        let html_elems = doc.get_elements_by_tag_name("html");
        assert!(!html_elems.is_empty());

        let p_elems = doc.get_elements_by_tag_name("p");
        assert_eq!(p_elems.len(), 1);
    }
}
