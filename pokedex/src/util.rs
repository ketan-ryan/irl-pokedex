pub type Rgb = [f64; 3];

pub trait RgbExt {
    fn from(color: iced::Color) -> Rgb;
}

impl RgbExt for Rgb {
    fn from(color: iced::Color) -> Rgb {
        [color.r.into(), color.g.into(), color.b.into()]
    }
}

/// Linearly interpolates between 2 RGB colors
/// # Arguments
/// * `a` The start color
/// * `b` The end color
/// * `t` The amount to interpolate by
///
/// # Returns
/// A color between the original colors at the specified point
fn lerp_color(a: &Rgb, b: &Rgb, t: f64) -> Rgb {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}
