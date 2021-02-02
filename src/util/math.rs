use crate::image::{Number, SubImage, BaseImage};
use crate::error::{ImgProcError, ImgProcResult};

/// Returns the result of the multiplication of a square matrix by a vector
pub fn vector_mul<T: Number>(mat: &[T], vec: &[T]) -> ImgProcResult<Vec<T>> {
    let rows = vec.len();
    let mat_cols = mat.len() / rows;

    // Check for valid dimensions
    if mat_cols != rows {
        return Err(ImgProcError::InvalidArgError("mat and vec dimensions do not match".to_string()));
    }

    let mut output = vec![0.into(); rows];

    for i in 0..rows {
        for j in 0..rows {
            output[i] += mat[rows * i + j] * vec[j];
        }
    }

    Ok(output)
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
    let num_channels = pixels.info().channels as usize;

    // Check for valid dimensions
    if size % 2 == 0 {
        return Err(ImgProcError::InvalidArgError("kernel length is not odd".to_string()));
    } else if kernel.len() != size {
        return Err(ImgProcError::InvalidArgError("pixels and kernel dimensions do not match".to_string()));
    }

    let mut vec = vec![0.0; num_channels];

    // Apply kernel
    for i in 0..size {
        for j in 0..num_channels {
            vec[j] += kernel[i] * pixels[i][j];
        }
    }

    Ok(vec)
}

/// Applies a 2D kernel to `pixels`
pub fn apply_2d_kernel(pixels: SubImage<f64>, kernel: &[f64]) -> ImgProcResult<Vec<f64>> {
    let size = pixels.info().width as usize;
    let num_channels = pixels.info().channels as usize;

    // Check for valid dimensions
    if size % 2 == 0 {
        return Err(ImgProcError::InvalidArgError("kernel dimensions are not odd".to_string()))
    } else if kernel.len() != size * size {
        return Err(ImgProcError::InvalidArgError("pixels and kernel dimensions do not match".to_string()));
    }

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