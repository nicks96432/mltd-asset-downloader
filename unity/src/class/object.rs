pub struct Object {
    object_hide_flags: u32,
}

impl Object {
    pub fn read() -> Self {
        Self {
            object_hide_flags: 0u32,
        }
    }
    
    
}
