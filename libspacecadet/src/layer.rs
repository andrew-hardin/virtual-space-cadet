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
    layers_to_disable_upon_release: Vec<(String, u64)>
}

impl LayerCollection {

    pub fn new() -> LayerCollection {
        LayerCollection {
            attributes: Vec::new(),
            name_to_idx: HashMap::new(),
            layers_to_disable_upon_release: Vec::new()
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
        println!("Setting {} to {}", name, val);
        self.attributes[self.name_to_idx[name]].enabled = val;
    }

    // TODO: should layer callbacks be relocated? What owns this responsibility?

    // Disable the given layer after the next key is released.
    pub fn disable_layer_after_release(&mut self, name: &str, target_release_count: u64) {
        // I'm pretty sure this capability shouldn't live here.
        self.layers_to_disable_upon_release.push((name.to_string(), target_release_count));
    }

    pub fn check_callbacks(&mut self, current_count: u64) {

        let mut to_disable = Vec::new();
        for i in 0..self.layers_to_disable_upon_release.len() {
            let count = self.layers_to_disable_upon_release[i].1;
            if count <= current_count {
                to_disable.push(i);
            }
        }

        let mut alter = 0;
        for mut i in to_disable {
            i += alter;
            let s = self.layers_to_disable_upon_release[i].0.clone();
            self.set(&s, false);
            self.layers_to_disable_upon_release.remove(i);
            alter += 1
        }
    }
}