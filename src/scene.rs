use cgmath::Vector3;
use crevice::std140::Writer;

use crate::shape::Quad;

// TODO: SCENE ONLY USES QUAD, MIGHT WANT MORE?
pub struct Scene {
    world: Vec<Quad>,
}

impl Scene {
    // Make a quad with coordinates, but in scene space, not clip space
    pub fn test() -> Self {
        Scene {
            world: vec![
                Quad::new(
                    Vector3::new(1.0, 0.0, 1.0),
                    Vector3::new(0.0, 0.0, 0.0),
                    Vector3::new(1.0, 1.0, 1.0),
                    Vector3::new(0.0, 1.0, 0.0),
                ),
                Quad::new(
                    Vector3::new(2.0, 1.0, 1.0),
                    Vector3::new(1.0, 1.0, 1.0),
                    Vector3::new(2.0, 2.0, 1.0),
                    Vector3::new(1.0, 2.0, 1.0),
                ),
            ],
        }
    }
    pub fn as_bytes(&self) -> [u8; 256] {
        let mut buffer = [0u8; 256];
        let mut writer = Writer::new(&mut buffer[..]);
        let _count = writer.write(self.world.as_slice()).unwrap();
        buffer
    }
}
