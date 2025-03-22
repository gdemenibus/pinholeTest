use cgmath::Vector3;

use crate::shape::Quad;

// TODO: SCENE ONLY USES QUAD, MIGHT WANT MORE?
#[derive(crevice::std140::AsStd140)]
pub struct Scene {
    world: Quad,
}

impl Scene {
    // Make a quad with coordinates, but in scene space, not clip space
    pub fn test() -> Self {
        Scene {
            world: Quad::new(
                Vector3::new(1.0, 0.0, 0.0),
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(1.0, 1.0, 0.0),
                Vector3::new(0.0, 1.0, 0.0),
            ),
        }
    }
}
