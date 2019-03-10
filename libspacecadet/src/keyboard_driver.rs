use crate::input_keyboard::*;
use crate::output_keyboard::*;
use crate::virtual_keyboard_matrix::*;
use crate::layer::*;
use crate::keys::*;
use std::time::{Duration, Instant};

/// A driver that includes in/out devices, a matrix, and key layers.
pub struct KeyboardDriver<I, O> where I: InputKeyboard, O: OutputKeyboard {
    pub input: I,
    pub output: O,
    pub matrix: VirtualKeyboardMatrix,
    pub layered_codes: Vec<KeyCodeMatrix>,
    pub layer_attributes: LayerCollection
}

impl<I, O> KeyboardDriver<I, O> where I: InputKeyboard, O: OutputKeyboard {

    /// Add a layer to the driver by specify its attributes and code matrix.
    pub fn add_layer(&mut self, attr: LayerAttributes, codes: KeyCodeMatrix) {
        self.layer_attributes.add(attr);
        self.layered_codes.push(codes);
    }

    pub fn clock_tick(&mut self, now: Instant) {

        // Before dispatching new events, check if any layers need to be disabled.
        self.layer_attributes.check_event_callbacks(self.output.get_stats());

        // Check for any keys that have been held down and oppressed by the user.
        // TODO: relocate constant to a config/params object.
        let hold_down_threshold = Duration::from_millis(250);
        for idx in self.matrix.detect_held_keys(hold_down_threshold, now) {
            self.matrix_state_changed(idx, KeyStateChange::Pressed, now);
        }

        // Handle every event coming in from the input device.
        for i in self.input.read_events() {
            match self.matrix.update(i.clone(), now) {
                // The key was within the matrix - store the update for later.
                MatrixUpdateResult::Bypass => { self.output.send_bypass_buffer(i); },
                MatrixUpdateResult::Redundant(_idx) => {},
                MatrixUpdateResult::StateChanged(idx, state) => { self.matrix_state_changed(idx, state, now); },
                MatrixUpdateResult::Blocked => {}
            }
        }
    }

    fn matrix_state_changed(&mut self, idx: Index2D, state: KeyStateChange, now: Instant) {

        // Starting at the highest enabled layer, find the first key that's
        // not transparent.
        let l = self.layered_codes.len();
        for i in (0..l).rev() {
            if self.layer_attributes.is_enabled(i) {
                let code = &mut self.layered_codes[i].codes[idx.0][idx.1];
                if !code.is_transparent() {
                    println!("Found key on layer {}", i);

                    // Capture references to the driver and layers - then ask the key to handle
                    // a state change event.
                    let mut context = KeyEventContext {
                        output_device: &mut self.output,
                        virtual_matrix: &mut self.matrix,
                        layers: &mut self.layer_attributes,
                        location: idx,
                        now
                    };
                    code.handle_event(&mut context, state);
                    return;
                }
            }
        }
        println!("Reached bottom of stack without hitting a key.");
    }

    /// Verify that the driver layers are compatible.
    pub fn verify(&self) -> Result<(), String> {
        self.verify_dims()?;
        self.verify_key_constraints()?;
        Ok(())
    }

    /// Verify that matrix and layers share the same dimensions.
    fn verify_dims(&self) -> Result<(), String> {
        let dim = self.matrix.dim();
        for i in self.layered_codes.iter().enumerate() {
            let other_dim = i.1.dim();
            if dim != other_dim {
                return Err(format!("Mismatched matrices- the virtual matrix is {}x{}, but layer \"{}\" (#{}) is {}x{}.",
                    dim.0, dim.1,
                    i.0, self.layer_attributes.attributes[i.0].name,
                    other_dim.0, other_dim.1));
            }
        }
        Ok(())
    }

    /// Verify that per-key constraints are satisfied.
    fn verify_key_constraints(&self) -> Result<(), String> {
        // Loop through every key in every layer.
        // Verify every constraint- quit early if a constraint is violated.
        for i in self.layered_codes.iter().enumerate() {
            for r in i.1.codes.iter().enumerate() {
                for c in r.1.iter().enumerate() {
                    let idx = (r.0, c.0);
                    for rule in c.1.get_constraints() {
                        self.verify_key_constraint(rule, idx, &self.layer_attributes.attributes[i.0].name)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Verify that a single key constraint is met.
    fn verify_key_constraint(&self, constraint: KeyConstraint, idx: Index2D, parent_layer: &str) -> Result<(), String>{
        match constraint {
            KeyConstraint::LayerExists(name) => {
                if !self.layer_attributes.name_to_idx.get(&name).is_some() {
                    Err(format!(
                        "Key constraint violated: the key at {}x{} on layer \"{}\" references \"{}\",\
                         but no layer exists with that name.",
                        idx.0, idx.1, parent_layer, name))
                } else {
                    Ok(())
                }
            }
            KeyConstraint::KeyOnOtherLayerIsTransparent(name) => {
                let layer_idx = *self.layer_attributes.name_to_idx.get(&name).unwrap();
                let other_key = &self.layered_codes[layer_idx].codes[idx.0][idx.1];
                if !other_key.is_transparent() {
                    Err(format!(
                        "Key constraint violated: the key at {}x{} on layer \"{}\" requires the key \
                         at {}x{} on \"{}\" to be transparent.",
                        idx.0, idx.1, parent_layer,
                        idx.0, idx.1, name))
                } else {
                    Ok(())
                }
            }
        }
    }
}






