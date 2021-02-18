//! A module for utility math functions

use crate::error;
use crate::image::{Number, SubImage, BaseImage};
use crate::error::ImgProcResult;

use std::f64::consts::{PI, E};
use rulinalg::matrix::{Matrix, BaseMatrix};

/// Returns the result of the multiplication of a square matrix by a vector
pub fn vector_mul<T: Number>(mat: &[T], vec: &[T]) -> ImgProcResult<Vec<T>> {
    let rows = vec.len();
    let mat_cols = mat.len() / rows;

    error::check_equal(mat_cols, rows, "mat and vec dimensions")?;

    let mut output = vec![0.into(); rows];

    for i in 0..rows {
        for j in 0..rows {
            output[i] += mat[rows * i + j] * vec[j];
        }
    }

    Ok(output)
}

/// If `kernel` is separable, returns the (vertical kernel, horizontal kernel); otherwise returns None
pub fn separate_kernel(kernel: &[f64]) -> Option<(Vec<f64>, Vec<f64>)> {
    let size = (kernel.len() as f32).sqrt() as usize;
    let kernel_mat = Matrix::new(size, size, kernel);
    let size = kernel_mat.cols();
    let (s, u, v) = kernel_mat.svd().unwrap();

    // Check if kernel is separable
    if s.data()[0] != 0.0 {
        for i in 1..size {
            if s.data()[i * size + i] != 0.0 {
                return None;
            }
        }
    } else {
        return None;
    }

    let scalar = s.data()[0].sqrt();
    let vertical_kernel = (u.col(0).as_slice().into_matrix() * scalar).into_vec();
    let horizontal_kernel = (v.transpose().row(0).as_slice().into_matrix() * scalar).into_vec();
    Some((vertical_kernel, horizontal_kernel))
}

/// Returns the maximum of three f64 values
pub fn max_3(x: f64, y: f64, z: f64) -> f64 {
    if x > y {
        if x > z {
            x
        } else {
            z
        }
    } else {
        if y > z {
            y
        } else {
            z
        }
    }
}

/// Returns the minimum of three f64 values
pub fn min_3(x: f64, y: f64, z: f64) -> f64 {
    if x < y {
        if x < z {
            x
        } else {
            z
        }
    } else {
        if y < z {
            y
        } else {
            z
        }
    }
}

/// Returns the maximum of four f64 values
pub fn max_4(w: f64, x: f64, y: f64, z: f64) -> f64 {
    if w > x {
        max_3(w, y, z)
    } else if x > y {
        max_3(w, x, z)
    } else if y > z {
        max_3(w, x, y)
    } else {
        max_3(x, y, z)
    }
}

/// Returns the minimum of four f64 values
pub fn min_4(w: f64, x: f64, y: f64, z: f64) -> f64 {
    if w < x {
        min_3(w, y, z)
    } else if x < y {
        min_3(w, x, z)
    } else if y < z {
        min_3(w, x, y)
    } else {
        min_3(x, y, z)
    }
}

/// Applies a 1D kernel to `pixels`
pub fn apply_1d_kernel(pixels: SubImage<f64>, kernel: &[f64]) -> ImgProcResult<Vec<f64>> {
    let size = pixels.info().size() as usize;
    let channels = pixels.info().channels as usize;

    error::check_odd(kernel.len(), "kernel length")?;
    error::check_equal(kernel.len(), size, "pixels and kernel dimensions")?;

    let mut vec = vec![0.0; channels];

    // Apply kernel
    for i in 0..size {
        for j in 0..channels {
            vec[j] += kernel[i] * pixels[i][j];
        }
    }

    Ok(vec)
}

/// Applies a 2D kernel to `pixels`
pub fn apply_2d_kernel(pixels: SubImage<f64>, kernel: &[f64]) -> ImgProcResult<Vec<f64>> {
    let size = pixels.info().width as usize;
    let num_channels = pixels.info().channels as usize;

    error::check_odd(kernel.len(), "kernel length")?;
    error::check_equal(kernel.len(), size * size, "pixels and kernel dimensions")?;

    let mut vec = vec![0.0; num_channels];

    // Apply kernel
    for y in 0..size {
        for x in 0..size {
            let index = y * size + x;
            for j in 0..num_channels {
                vec[j] += kernel[index] * pixels[index][j];
            }
        }
    }

    Ok(vec)
}

/// Calculates the distance between two points
pub fn distance(x_1: u32, y_1: u32, x_2: u32, y_2: u32) -> f64 {
    let x_dist = (x_1 as f64) - (x_2 as f64);
    let y_dist = (y_1 as f64) - (y_2 as f64);

    ((x_dist * x_dist) + (y_dist * y_dist)).sqrt()
}

/// Calculates the Gaussian function for G_sigma(x)
pub fn gaussian_fn(x: f64, sigma: f64) -> ImgProcResult<f64> {
    error::check_non_neg(sigma, "sigma")?;

    let sigma_squared = sigma * sigma;

    Ok((1.0 / (2.0 * PI * sigma_squared)) * E.powf(-(x * x) / (2.0 * sigma_squared)))
}

/// Cubic weighting function for bicubic interpolation
pub fn cubic_weighting_fn(x: f64) -> f64 {
    (1.0 / 6.0) * (clamp_zero(x + 2.0).powf(3.0)
        - 4.0 * clamp_zero(x + 1.0).powf(3.0)
        + 6.0 * clamp_zero(x).powf(3.0)
        - 4.0 * clamp_zero(x - 1.0).powf(3.0))
}

/// Creates a lower bound of zero
pub fn clamp_zero(x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }

    x
}

/// Clamps `x` in the range from `min` to `max`, inclusive
pub fn clamp<T: Number>(x: T, min: T, max: T) -> T {
    if x <= min {
        return min;
    }

    if x >= max {
        return max;
    }

    x
}

/// If `x` is greater than `max`, returns `max`; otherwise returns `x`
pub fn clamp_max<T: Number>(x: T, max: T) -> T {
    if x >= max {
        return max;
    }

    return x;
}

/// Normalized sinc function
pub fn sinc_norm(x: f64) -> f64 {
    if x == 0.0 {
        return 1.0;
    }

    let pi_x = PI * x;

    pi_x.sin() / pi_x
}

/// Lanczos kernel
pub fn lanczos_kernel(x: f64, a: f64) -> f64 {
    if x > -a && x < a {
        return sinc_norm(x) * sinc_norm(x / a);
    }

    0.0
}