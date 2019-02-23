use crate::virtual_keyboard_matrix::Index2D;
use crate::keys::*;
use crate::virtual_keyboard_matrix::KeyStateChange;

pub struct KeyCodeMatrix {
    dim: Index2D,
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
            dim,
            codes,
        }
    }
}


pub struct Layer {
    pub name: String,
    pub enabled: bool,
    pub codes: KeyCodeMatrix,
}

impl Layer {

}