// File for loading shaders 
use std::fs::File;
use std::io::Read;

// Load shader from path.
pub fn load_shader(path: &str) -> String {
    let mut file = File::open(path).expect("Failed to open shader file");
    let mut source = String::new();
    file.read_to_string(&mut source).expect("Failed to read shader file");
    source
}



