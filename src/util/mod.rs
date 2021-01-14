pub mod math;
pub mod constants;

use crate::image::Image;

use std::collections::{HashMap, BTreeMap};

// Trait for valid image channel types
pub trait Number:
    std::clone::Clone
    + std::marker::Copy
    + std::fmt::Display
    + std::ops::Add<Output=Self>
    + std::ops::Sub<Output=Self>
    + std::ops::Mul<Output=Self>
    + std::ops::Div<Output=Self>
    + std::ops::AddAssign
    + std::ops::SubAssign
    + std::ops::MulAssign
    + std::ops::DivAssign
    + From<u8>
    where Self: std::marker::Sized {}

impl<T> Number for T
    where T:
        std::clone::Clone
        + std::marker::Copy
        + std::fmt::Display
        + std::ops::Add<Output=T>
        + std::ops::Sub<Output=T>
        + std::ops::Mul<Output=T>
        + std::ops::Div<Output=T>
        + std::ops::AddAssign
        + std::ops::SubAssign
        + std::ops::MulAssign
        + std::ops::DivAssign
        + From<u8>
{}

// Image helper functions

pub fn generate_xyz_tristimulus_vals(ref_white: &str) -> Option<(f32, f32, f32)> {
    return match ref_white.to_lowercase().as_str() {
        "d50" => Some((96.4212, 100.0, 82.5188)),
        "d65" => Some((95.0489, 100.0, 103.8840)),
        _ => None,
    }
}

pub fn xyz_to_lab_fn(num: f32) -> f32 {
    let d: f32 = 6.0 / 29.0;

    if num > d.powf(3.0) {
        num.powf(1.0 / 3.0)
    } else {
        (num / (3.0 * d * d)) + (4.0 / 29.0)
    }
}

pub fn lab_to_xyz_fn(num: f32) -> f32 {
    let d: f32 = 6.0 / 29.0;

    if num > d {
        num.powf(3.0)
    } else {
        3.0 * d * d * (num - (4.0 / 29.0))
    }
}

// Input: image in CIELAB
pub fn generate_histogram_percentiles(input: &Image<f32>, percentiles: &mut HashMap<i32, f32>, precision: f32) {
    let mut histogram = BTreeMap::new();
    let (width, height) = input.dimensions();

    for y in 0..height {
        for x in 0..width {
            let p = (input.get_pixel(x, y).channels()[0] * precision).round() as i32;
            let count = histogram.entry(p).or_insert(1);
            *count += 1;
        }
    }

    let mut sum: i32 = 0;
    let num_pixels = (width * height) as f32;
    for (key, val) in &histogram {
        sum += val;
        percentiles.insert(*key, sum as f32 / num_pixels);
    }
}

pub fn create_lookup_table<T: Number, F>(table: &mut [T; 256], f: F)
    where F: Fn(u8) -> T {
    for i in 0..256 {
        table[i] = f(i as u8);
    }
}

// Convert an image from f32 [0, 1] to u8 [0,255]
pub fn image_f32_to_u8(input: &Image<f32>) -> Image<u8> {
    input.map_channels(|channel| (channel * 255.0).round() as u8)
}