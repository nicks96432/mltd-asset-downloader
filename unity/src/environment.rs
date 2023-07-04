use crate::class::Class;

use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Environment {
    pub classes: HashMap<String, Box<dyn Class>>,
}

impl Environment {
    pub fn new() -> Self {
        Self::default()
    }
}
