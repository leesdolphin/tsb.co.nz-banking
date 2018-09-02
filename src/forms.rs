use std::io::Error;

use html5ever::driver::ParseOpts;
use html5ever::parse_document;
use html5ever::rcdom::{Handle, NodeData, RcDom};
use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::TreeBuilderOpts;

use std::collections::VecDeque;

pub enum FormElement {
  Input {
    name: Option<String>,
    id: Option<String>,
    value: Option<String>,
  },
}

pub struct IterNodes {
  to_explore: VecDeque<Handle>,
}

impl IterNodes {
  pub fn from(node: &Handle) -> Self {
    let mut to_explore: VecDeque<Handle> = VecDeque::new();
    to_explore.push_front(node.clone());
    IterNodes {
      to_explore: to_explore,
    }
  }
}

impl Iterator for IterNodes {
  type Item = Handle;
  fn next(&mut self) -> Option<Self::Item> {
    let te = &mut self.to_explore;
    if let Some(node) = te.pop_front() {
      {
        let children = node.children.borrow();
        for i in 0..children.len() {
          if let Some(node) = children.get(i) {
            te.insert(i, node.clone());
          }
        }
      }
      Some(node)
    } else {
      None
    }
  }
}

pub fn get_text_content(node: &Handle) -> String {
  let mut r = "".to_owned();
  for node in IterNodes::from(node) {
    match node.data {
      NodeData::Text { ref contents } => {
        r.push(' ');
        r.push_str(&contents.borrow().to_string().trim());
      }
      _ => {}
    }
  }
  r.trim()
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
    match get_node_name(&node) {
      Some(node_name) => match node_name.as_str() {
        "input" => {
          let mut a_name: Option<String> = get_attr(&node, "name");
          let mut a_id: Option<String> = get_attr(&node, "id");
          let mut a_value: Option<String> = get_attr(&node, "value");
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
    match get_node_name(&node) {
      Some(node_name) => match node_name.as_str() {
        "form" => {
          r.push(node);
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
