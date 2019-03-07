use std::collections::HashMap;
use crate::virtual_keyboard_matrix::Index2D;
use crate::keys::*;
use crate::virtual_keyboard_matrix::{KeyStats, KeyStateChange};

pub struct KeyCodeMatrix {
    pub codes: Vec<Vec<Box<KeyCode>>>,
}

impl KeyCodeMatrix {

    pub fn new(dim: Index2D) -> KeyCodeMatrix {
        let mut codes: Vec<Vec<Box<KeyCode>>> = Vec::with_capacity(dim.0);
        for _r in 0..dim.0 {
            let mut row: Vec<Box<KeyCode>> = Vec::with_capacity(dim.1);
            for _c in 0..dim.1 {
                row.push(Box::new(TransparentKey {}));
            }
            codes.push(row);
        }

        KeyCodeMatrix {
            codes,
        }
    }

    pub fn dim(&self) -> Index2D {
        let rows = self.codes.len();
        if rows > 0 {
            (rows, self.codes[0].len())
        } else {
            (rows, 0)
        }
    }
}

pub struct LayerAttributes {
    pub name: String,
    pub enabled: bool
}

pub struct LayerCollection {
    pub attributes: Vec<LayerAttributes>,
    pub name_to_idx: HashMap<String, usize>,
    event_layer_callbacks: Vec<ScheduledLayerEvent>
}

pub struct ScheduledLayerEvent {
    pub layer_name: String,
    pub event_type: KeyStateChange,
    pub event_count: u32,
    pub enable_layer_at_event: bool
}

impl LayerCollection {

    pub fn new() -> LayerCollection {
        LayerCollection {
            attributes: Vec::new(),
            name_to_idx: HashMap::new(),
            event_layer_callbacks: Vec::new()
        }
    }

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
        println!("Layer \"{}\" = {}", name, val);
        self.attributes[self.name_to_idx[name]].enabled = val;
    }

    // Schedule a layer related event.
    pub fn schedule_event_count_callback(&mut self, e: ScheduledLayerEvent) {
        self.event_layer_callbacks.push(e);
    }

    // Check if any layer related events have occurred.
    pub fn check_event_callbacks(&mut self, state: KeyStats) {

        // This function could be replaced by drain_filter, but it's a nightly-only experiment.
        let mut to_change: Vec<(String, bool)> = Vec::new();
        self.event_layer_callbacks.retain(|x| {
            let ready = x.event_count <= state.get(x.event_type);
            if ready {
                to_change.push((x.layer_name.clone(), x.enable_layer_at_event));
            }
            !ready
        });

        for (name, state) in to_change {
            self.set(&name, state);
        }
    }
}