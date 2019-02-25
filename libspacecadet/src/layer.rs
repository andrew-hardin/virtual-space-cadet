use std::collections::HashMap;
use crate::virtual_keyboard_matrix::Index2D;
use crate::keys::*;

pub struct KeyCodeMatrix {
    pub codes: Vec<Vec<Box<KeyCode>>>,
}

impl KeyCodeMatrix {

    pub fn new(dim: Index2D) -> KeyCodeMatrix {
        let mut codes: Vec<Vec<Box<KeyCode>>> = Vec::with_capacity(dim.0);
        for _r in 0..dim.0 {
            let mut row: Vec<Box<KeyCode>> = Vec::with_capacity(dim.1);
            for _c in 0..dim.1 {
                row.push(Box::new(BlankKey {}));
            }
            codes.push(row);
        }

        KeyCodeMatrix {
            codes,
        }
    }
}

pub struct LayerAttributes {
    pub name: String,
    pub enabled: bool
}

pub struct LayerCollection {
    pub attributes: Vec<LayerAttributes>,
    name_to_idx: HashMap<String, usize>,
}

impl LayerCollection {

    pub fn new() -> LayerCollection {
        LayerCollection {
            attributes: Vec::new(),
            name_to_idx: HashMap::new()
        }
    }

    // TODO: check best practices for passing the most general string to a fn.
    pub fn add(&mut self, attr: LayerAttributes) {
        self.name_to_idx.insert(attr.name.clone(), self.attributes.len());
        self.attributes.push(attr);
    }

    pub fn len(&self) -> usize {
        self.attributes.len()
    }

    pub fn is_enabled(&self, idx: usize) -> bool {
        self.attributes[idx].enabled
    }

    pub fn toggle(&mut self, name: &str) {
        let v = &mut self.attributes[self.name_to_idx[name]].enabled;
        *v = !*v;
    }

    pub fn set(&mut self, name: &str, val: bool) {
        self.attributes[self.name_to_idx[name]].enabled = val;
    }
}