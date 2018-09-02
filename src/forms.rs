use std::borrow::Borrow;
use std::io::Error;
use std::slice::Iter;

use html5ever::driver::ParseOpts;
use html5ever::parse_document;
use html5ever::rcdom::{Handle, NodeData, RcDom};
use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::TreeBuilderOpts;

pub enum FormElement {
  Input {
    name: Option<String>,
    id: Option<String>,
    value: Option<String>,
  },
}

pub struct IterNodes<'a> {
  start: Option<&'a Handle>,
  tree: Vec<&'a mut Iter<'a, Handle>>,
}

impl<'a> IterNodes<'a> {
  pub fn from(node: &'a Handle) -> Self {
    IterNodes {
      start: Some(node),
      tree: vec![],
    }
  }

  fn descend_node(&mut self, node: &'a Handle) {
    // self.tree.push(node.children.borrow().iter());
  }
}

impl<'a> Iterator for IterNodes<'a> {
  type Item = &'a Handle;
  fn next(&mut self) -> Option<Self::Item> {
    if let Some(node) = self.start {
      self.descend_node(node);
      self.start = None;
      Some(node)
    } else {
      match self.tree.last() {
        Some(mut iter) => match iter.next() {
          Some(node) => return Some(node),
          None => {}
        },
        None => return None,
      }
      self.tree.pop();
      self.next()
    }
  }
}

pub fn get_attr(node: &Handle, name: &str) -> Option<String> {
  match node.data {
    NodeData::Element { ref attrs, .. } => {
      for attr in attrs.borrow().iter() {
        if attr.name.ns == ns!() && name == attr.name.local.to_string().as_str() {
          return Some(attr.value.to_string());
        }
      }
      None
    }
    _ => None,
  }
}

pub fn get_node_name(node: &Handle) -> Option<String> {
  match node.data {
    NodeData::Element { ref name, .. } => match name.ns {
      ns!(html) => Some(name.local.to_string().to_lowercase()),
      _ => None,
    },
    _ => None,
  }
}

pub fn find_inputs(node: &Handle) -> Vec<FormElement> {
  let mut r: Vec<FormElement> = vec![];
  for node in IterNodes::from(node) {
    match get_node_name(node) {
      Some(node_name) => match node_name.as_str() {
        "input" => {
          let mut a_name: Option<String> = get_attr(node, "name");
          let mut a_id: Option<String> = get_attr(node, "id");
          let mut a_value: Option<String> = get_attr(node, "value");
          r.push(FormElement::Input {
            name: a_name,
            id: a_id,
            value: a_value,
          });
        }
        _ => {}
      },
      _ => {}
    }
  }
  r
}

pub fn find_forms(node: &Handle) -> Vec<Handle> {
  let mut r: Vec<Handle> = vec![];
  for node in IterNodes::from(node) {
    match get_node_name(node) {
      Some(node_name) => match node_name.as_str() {
        "form" => {
          r.push(node.clone());
        }
        _ => {}
      },
      _ => {}
    }
  }
  r
}

pub fn parse_dom(content: String) -> Result<Handle, Error> {
  let opts = ParseOpts {
    tree_builder: TreeBuilderOpts {
      drop_doctype: true,
      ..Default::default()
    },
    ..Default::default()
  };
  let mut data: &[u8] = content.as_bytes();
  parse_document(RcDom::default(), opts)
    .from_utf8()
    .read_from(&mut data)
    .map(|dom| dom.document)
}
