use glium::{glutin::surface::WindowSurface, texture::Texture2d, Display};
// File in charge of handling texture loading and such
//
pub fn load_texture(path: String, display: &Display<WindowSurface>) -> Texture2d {
    let img = image::open(path).unwrap().to_rgba8();

    let image_dimensions = img.dimensions();

    let image = glium::texture::RawImage2d::from_raw_rgba_reversed(&img.into_raw(), image_dimensions);

    Texture2d::new(display, image).unwrap()
}
