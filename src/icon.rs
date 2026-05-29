use ab_glyph::{FontRef, PxScale};
use image::{ImageBuffer, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use tray_icon::Icon;

const ICON_SIZE: u32 = 256;

pub fn create_text_icon(text: &str) -> Result<Icon, Box<dyn std::error::Error>> {
    let mut image: RgbaImage = ImageBuffer::from_pixel(ICON_SIZE, ICON_SIZE, Rgba([0, 0, 0, 0]));
    let font_data = include_bytes!("../assets/DejaVuSans.ttf");
    let font = FontRef::try_from_slice(font_data).map_err(|_| "failed to load embedded font")?;
    let scale = text_scale(text);
    let (x, y) = text_position(text);

    draw_text_mut(
        &mut image,
        Rgba([255_u8, 255_u8, 255_u8, 255_u8]),
        x,
        y,
        scale,
        &font,
        text,
    );

    Icon::from_rgba(image.into_raw(), ICON_SIZE, ICON_SIZE)
        .map_err(|error| Box::new(error) as Box<dyn std::error::Error>)
}

fn text_scale(text: &str) -> PxScale {
    match text.chars().count() {
        0..=2 => PxScale::from(200.0),
        3 => PxScale::from(110.0),
        _ => PxScale::from(80.0),
    }
}

fn text_position(text: &str) -> (i32, i32) {
    match text.chars().count() {
        0..=2 => (40, 40),
        3 => (30, 40),
        _ => (20, 40),
    }
}
