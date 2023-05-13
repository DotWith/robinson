use robinson_css::{Value, StyleSheet, CssRule, Selector, SimpleSelector, Specificity, NormalRule};
use robinson_dom::{Node, Element};
use std::{cell::RefCell, rc::Rc, collections::HashMap};

pub type PropertyMap = HashMap<String, Value>;

#[derive(Debug)]
pub enum Display {
    Block,
    Inline,
    InlineBlock,
    Table,
    TableRowGroup,
    TableRow,
    TableCell,
    ListItem,
    None,
}

#[derive(Debug)]
pub struct StyleNode {
    pub node: Node,
    pub specified_values: PropertyMap,
    pub children: RefCell<Vec<Rc<StyleNode>>>,
}

#[derive(Debug)]
pub struct StyleTree {
    pub root: RefCell<Rc<StyleNode>>,
}

impl StyleNode {
    pub fn new(node: &Node, stylesheets: &Vec<StyleSheet>) -> Rc<Self> {
        Rc::new(Self {
            node: node.clone(),
            specified_values: match node {
                Node::Element(elem) => specified_values(elem, stylesheets),
                Node::Text(_) | Node::Comment(_) => HashMap::new()
            },
            children: RefCell::new(node
                .element()
                .map(|element| {
                    element
                        .children
                        .iter()
                        .map(|child| Self::new(child, stylesheets))
                        .collect()
                })
                .unwrap_or_else(Vec::new),
            ),
        })
    }

    pub fn get_value(&self, name: &str) -> Option<Value> {
        self.specified_values.get(name).cloned()
    }

    pub fn lookup(&self, name: &str, fallback_name: &str, default: &Value) -> Value {
        self.get_value(name)
            .or_else(|| self.get_value(fallback_name))
            .unwrap_or_else(|| default.clone())
    }

    pub fn display(&self) -> Display {
        if matches!(self.node, Node::Text(_)) {
            return Display::Inline;
        }

        self.get_value("display")
            .and_then(|value| match value {
                Value::Keyword(s) => Some(match &*s {
                    "block" => Display::Block,
                    "none" => Display::None,
                    "inline-block" => Display::InlineBlock,
                    "table" => Display::Table,
                    "table-row-group" => Display::TableRowGroup,
                    "table-row" => Display::TableRow,
                    "table-cell" => Display::TableCell,
                    "list-item" => Display::ListItem,
                    _ => Display::Inline,
                }),
                _ => None
            })
            .unwrap_or(Display::Inline)
    }
}

impl StyleTree {
    pub fn new(node: &Node, stylesheets: &Vec<StyleSheet>) -> Self {
        Self {
            root: RefCell::new(StyleNode::new(node, stylesheets))
        }
    }
}

/// Apply styles to a single element, returning the specified styles.
fn specified_values(elem: &Element, stylesheets: &Vec<StyleSheet>) -> PropertyMap {
    let mut values = HashMap::new();
    let mut rules = Vec::new();
    for stylesheet in stylesheets {
        rules.extend(matching_rules(elem, stylesheet));
    }

    // Sort the matched rules by specificity, highest to lowest.
    rules.sort_by(|&(a, _), &(b, _)| b.cmp(&a));

    for (_, rule) in rules {
        for declaration in &rule.declarations {
            values.insert(declaration.0.clone(), declaration.1.clone());
        }
    }
    values
}

/// A single CSS rule and the specificity of its most specific matching selector.
type MatchedRule<'a> = (Specificity, &'a NormalRule);

/// Find all CSS rules that match the given element.
fn matching_rules<'a>(elem: &Element, stylesheet: &'a StyleSheet) -> Vec<MatchedRule<'a>> {
    // For now, we just do a linear scan of all the rules.  For large
    // documents, it would be more efficient to store the rules in hash tables
    // based on tag name, id, class, etc.
    stylesheet
        .rules
        .iter()
        .flat_map(|rule| match rule {
            CssRule::Normal(norm) => Some(norm),
            _ => None
        })
        .filter_map(|rule| match_rule(elem, rule).map(|spec| (spec, rule)))
        .collect()
}

/// If `rule` matches `elem`, return a `MatchedRule`. Otherwise return `None`.
fn match_rule(elem: &Element, rule: &NormalRule) -> Option<Specificity> {
    // Find the first (most specific) matching selector.
    rule.selectors
        .iter()
        .find(|selector| matches(elem, selector))
        .map(|selector| selector.specificity().unwrap())
}

/// Selector matching:
fn matches(elem: &Element, selector: &Selector) -> bool {
    match *selector {
        Selector::Simple(ref simple_selector) => matches_simple_selector(elem, simple_selector)
    }
}

fn matches_simple_selector(elem: &Element, selector: &SimpleSelector) -> bool {
    // Check type selector
    if selector.tag_name.iter().any(|name| elem.name != *name) {
        return false
    }

    // Check ID selector
    if selector.id.iter().any(|id| elem.id != Some(id.to_string())) {
        return false;
    }

    // Check class selectors
    let elem_classes = &elem.classes;
    if selector.class.iter().any(|class| !elem_classes.contains(class)) {
        return false;
    }

    // We didn't find any non-matching selector components.
    true
}
