use std::collections::HashMap;
use crate::virtual_keyboard_matrix::Index2D;
use crate::keys::*;
use crate::virtual_keyboard_matrix::{KeyStats, KeyStateChange};

/// A `MxN` matrix of boxed key code traits.
pub struct KeyCodeMatrix {
    pub codes: Vec<Vec<Box<KeyCode>>>,
}

impl KeyCodeMatrix {
    /// Create a new matrix of the given size.
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

    /// Get the size of the matrix.
    pub fn dim(&self) -> Index2D {
        let rows = self.codes.len();
        if rows > 0 {
            (rows, self.codes[0].len())
        } else {
            (rows, 0)
        }
    }
}

/// Attributes of a layer (e.g. name).
pub struct LayerAttributes {
    pub name: String,
    pub enabled: bool
}

/// A collection of layer attributes.
pub struct LayerCollection {
    pub attributes: Vec<LayerAttributes>,
    pub name_to_idx: HashMap<String, usize>,
    event_layer_callbacks: Vec<ScheduledLayerEvent>
}

/// A layer event that occurs when event counts pass a threshold.
pub struct ScheduledLayerEvent {
    pub layer_name: String,
    pub event_type: KeyStateChange,
    pub event_count: u32,
    pub enable_layer_at_event: bool
}

impl LayerCollection {

    /// Create a new empty layer collection.
    pub fn new() -> LayerCollection {
        LayerCollection {
            attributes: Vec::new(),
            name_to_idx: HashMap::new(),
            event_layer_callbacks: Vec::new()
        }
    }

    /// Add a layer to the collection.
    pub fn add(&mut self, attr: LayerAttributes) {
        self.name_to_idx.insert(attr.name.clone(), self.attributes.len());
        self.attributes.push(attr);
    }

    /// Count the number of items in the layer.
    pub fn len(&self) -> usize {
        self.attributes.len()
    }

    /// Determine if a layer is enabled.
    pub fn is_enabled(&self, idx: usize) -> bool {
        self.attributes[idx].enabled
    }

    /// Toggle a layer.
    pub fn toggle(&mut self, name: &str) {
        let v = &mut self.attributes[self.name_to_idx[name]].enabled;
        *v = !*v;
    }

    /// Set a layer state by name.
    pub fn set(&mut self, name: &str, val: bool) {
        self.attributes[self.name_to_idx[name]].enabled = val;
    }

    /// Schedule a layer related event.
    pub fn schedule_event_count_callback(&mut self, e: ScheduledLayerEvent) {
        self.event_layer_callbacks.push(e);
    }

    /// Check if any layer related events have occurred.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_code_matrix_dims() {
        let zero = (0, 0);
        let non_zero = (1, 2);
        assert_eq!(KeyCodeMatrix::new(zero).dim(), zero);
        assert_eq!(KeyCodeMatrix::new(non_zero).dim(), non_zero);
    }

    #[test]
    fn layer_collection_get_set() {
        let mut item = LayerCollection::new();
        let to_add = LayerAttributes {
            name: "foo".to_string(),
            enabled: true
        };
        item.add(to_add);
        assert_eq!(item.len(), 1);
        assert!(item.is_enabled(0));
        item.set("foo", false);
        assert!(!item.is_enabled(0));
        item.toggle("foo");
        assert!(item.is_enabled(0));
    }

    #[test]
    fn layer_collection_event_count_callback() {
        let mut item = LayerCollection::new();
        let to_add = LayerAttributes {
            name: "foo".to_string(),
            enabled: true
        };
        item.add(to_add);

        // Schedule an two event call backs.
        item.schedule_event_count_callback(ScheduledLayerEvent {
            layer_name: "foo".to_string(),
            event_type: KeyStateChange::Pressed,
            event_count: 10,
            enable_layer_at_event: false
        });
        item.schedule_event_count_callback(ScheduledLayerEvent {
            layer_name: "foo".to_string(),
            event_type: KeyStateChange::Released,
            event_count: 5,
            enable_layer_at_event: true
        });

        // Manipulate the stats and check that layers turn off and back on.
        let mut s = KeyStats::new();
        item.check_event_callbacks(s);
        assert!(item.is_enabled(0));
        s.set(KeyStateChange::Pressed, 10);
        item.check_event_callbacks(s);
        assert!(!item.is_enabled(0));
        s.set(KeyStateChange::Released, 6);
        item.check_event_callbacks(s);
        assert!(item.is_enabled(0));
    }
}