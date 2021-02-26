use crate::error;
use crate::error::{ImgProcResult, ImgProcError};
use crate::image::{Image, BaseImage};

use std::cmp::Reverse;

/// Applies a median filter, where each output pixel is the median of the pixels in a
/// `(2 * radius + 1) x (2 * radius + 1)` kernel in the input image. Based on Ben Weiss' partial
/// histogram method, using a tier radix of 2. For a detailed description, see:
/// http://citeseerx.ist.psu.edu/viewdoc/download?doi=10.1.1.93.1608&rep=rep1&type=pdf
pub fn median_filter(input: &Image<u8>, radius: u32) -> ImgProcResult<Image<u8>> {
    let mut n_cols = (4.0 * (radius as f64).powf(2.0 / 3.0)).floor() as usize;
    if n_cols % 2 == 0 {
        n_cols += 1;
    }

    let mut output = Image::blank(input.info());

    for x in (0..output.info().width).step_by(n_cols) {
        process_cols_med(input, &mut output, radius, n_cols, x);
    }

    Ok(output)
}

/// Applies an alpha-trimmed mean filter, where each output pixel is the alpha-trimmed mean of the
/// pixels in a `(2 * radius + 1) x (2 * radius + 1)` kernel in the input image
pub fn alpha_trimmed_mean_filter(input: &Image<u8>, radius: u32, alpha: u32) -> ImgProcResult<Image<u8>> {
    let size = 2 * radius + 1;
    error::check_even(alpha, "alpha")?;
    if alpha >= (size * size) {
        return Err(ImgProcError::InvalidArgError(format!("invalid alpha: size is {}, but alpha is {}", size, alpha)));
    }

    let mut n_cols = (4.0 * (radius as f64).powf(2.0 / 3.0)).floor() as usize;
    if n_cols % 2 == 0 {
        n_cols += 1;
    }

    let mut output = Image::blank(input.info());

    for x in (0..output.info().width).step_by(n_cols) {
        process_cols_mean(input, &mut output, radius, alpha, n_cols, x);
    }

    Ok(output)
}

#[derive(Debug, Clone)]
struct PartialHistograms {
    data: Vec<[i32; 256]>, // The partial histograms
    n_cols: usize,
    n_half: usize,
    radius: usize,
    size: usize,
}

impl PartialHistograms {
    fn new(radius: usize, n_cols: usize) -> Self {
        let size = (2 * radius + 1) as usize;
        let n_half = n_cols / 2;

        PartialHistograms {
            data: vec![[0; 256]; n_cols],
            n_cols,
            n_half,
            radius,
            size,
        }
    }

    fn update(&mut self, p_in: &Vec<&[u8]>, channel_index: usize, add: bool) {
        let mut inc = 1;
        if !add {
            inc *= -1;
        }

        // Update partial histograms
        for n in 0..self.n_half {
            let n_upper = self.n_cols - n - 1;

            for i in n..self.n_half {
                self.data[n][p_in[i][channel_index] as usize] += inc;
                self.data[n][p_in[i+self.size][channel_index] as usize] -= inc;

                let i_upper = self.n_cols + 2 * self.radius - i - 1;
                let i_lower = i_upper - self.size;
                self.data[n_upper][p_in[i_lower][channel_index] as usize] -= inc;
                self.data[n_upper][p_in[i_upper][channel_index] as usize] += inc;
            }
        }

        // Update central histogram
        for i in self.n_half..(self.n_half + self.size) {
            self.data[self.n_half][p_in[i][channel_index] as usize] += inc;
        }
    }

    fn get_count(&self, key: usize, index: usize) -> i32 {
        let mut count = self.data[self.n_half][key as usize];
        if index != self.n_half {
            count += self.data[index][key as usize];
        }

        count as i32
    }
}

////////////////////////////
// Median filter functions
////////////////////////////

#[derive(Debug, Clone)]
struct MedianHist {
    data: PartialHistograms,
    sums: Vec<i32>, // Sums to keep track of the number of values less than the previous median
    pivots: Vec<u8>, // Previous medians to act as "pivots" to find the next median
}

impl MedianHist {
    fn new(radius: usize, n_cols: usize) -> Self {
        MedianHist {
            data: PartialHistograms::new(radius, n_cols),
            sums: vec![0; n_cols],
            pivots: Vec::with_capacity(n_cols),
        }
    }

    fn data(&self) -> &PartialHistograms {
        &self.data
    }

    fn sums(&self) -> &[i32] {
        &self.sums
    }

    fn pivots(&self) -> &[u8] {
        &self.pivots
    }

    fn init_pivots(&mut self) {
        self.pivots = vec![0; self.data.n_cols];
    }

    fn set_pivot(&mut self, pivot: u8, index: usize) {
        self.pivots[index] = pivot;
    }

    fn set_sum(&mut self, sum: i32, index: usize) {
        self.sums[index] = sum;
    }

    fn update(&mut self, p_in: &Vec<&[u8]>, channel_index: usize, add: bool) {
        self.data.update(p_in, channel_index, add);

        let mut inc = 1;
        if !add {
            inc *= -1;
        }

        // Update sums
        if !self.pivots.is_empty() {
            for n in 0..self.data.n_cols {
                for i in n..(n + self.data.size) {
                    if p_in[i][channel_index] < self.pivots[n] {
                        self.sums[n] += inc;
                    }
                }
            }
        }
    }
}

fn process_cols_med(input: &Image<u8>, output: &mut Image<u8>, radius: u32, n_cols: usize, x: u32) {
    let size = 2 * radius + 1;
    let center = ((size * size) / 2 + 1) as i32;
    let (width, height, channels) = input.info().whc();
    let mut histograms = vec![MedianHist::new(radius as usize, n_cols); channels as usize];

    // Initialize histogram and process first row
    init_cols_med(input, output, &mut histograms, radius, center, n_cols, x);

    // Update histogram and process remaining rows
    for j in 1..height {
        // Update histograms
        let mut p_in = Vec::with_capacity(n_cols);
        let mut p_out = Vec::with_capacity(n_cols);
        let j_in = (j + radius).clamp(0, input.info().height - 1);
        let j_out = (j as i32 - radius as i32 - 1).clamp(0, input.info().height as i32 - 1) as u32;

        for i in (x as i32 - radius as i32)..((x + n_cols as u32 + radius) as i32) {
            let i_clamp = i.clamp(0, width as i32 - 1) as u32;
            p_in.push(input.get_pixel_unchecked(i_clamp, j_in));
            p_out.push(input.get_pixel_unchecked(i_clamp, j_out));
        }

        add_row_med(&mut histograms, &p_in, channels as usize);
        remove_row_med(&mut histograms, &p_out, channels as usize);

        process_row_med(output, &mut histograms, center, n_cols, x, j);
    }
}

fn init_cols_med(input: &Image<u8>, output: &mut Image<u8>, histograms: &mut Vec<MedianHist>, radius: u32, center: i32, n_cols: usize, x: u32) {
    let (width, height, channels) = input.info().whc();

    // Initialize histograms
    for j in -(radius as i32)..(radius as i32 + 1) {
        let mut p_in = Vec::with_capacity(n_cols);
        for i in (x as i32 - radius as i32)..((x + n_cols as u32 + radius) as i32) {
            p_in.push(input.get_pixel_unchecked(i.clamp(0, width as i32 - 1) as u32,
                                                j.clamp(0, height as i32 - 1) as u32));
        }

        add_row_med(histograms, &p_in, channels as usize);
    }

    // Initialize histogram pivots
    for c in 0..(channels as usize) {
        histograms[c].init_pivots();
    }

    // Compute first median values
    for i in 0..n_cols {
        let mut p_out = Vec::with_capacity(channels as usize);
        for c in 0..(channels as usize) {
            let mut sum = 0;

            for key in 0u8..=255 {
                let add = histograms[c].data().get_count(key as usize, i);

                if sum + add >= center {
                    p_out.push(key);
                    histograms[c].set_sum(sum, i);
                    break;
                }

                sum += add;
            }
        }

        let x_clamp = (x + i as u32).clamp(0, output.info().width - 1);
        output.set_pixel(x_clamp, 0, &p_out);

        set_pivots_med(histograms, &p_out, i);
    }
}

fn process_row_med(output: &mut Image<u8>, histograms: &mut Vec<MedianHist>, center: i32, n_cols: usize, x: u32, y: u32) {
    let channels = output.info().channels as usize;

    for i in 0..n_cols {
        let mut p_out = Vec::with_capacity(channels);
        for c in 0..channels {
            let pivot = histograms[c].pivots()[i];
            let mut sum = histograms[c].sums()[i];

            if sum < center {
                for key in pivot..=255 {
                    let add = histograms[c].data().get_count(key as usize, i);

                    if sum + add >= center {
                        p_out.push(key);
                        histograms[c].set_sum(sum, i);
                        break;
                    }

                    sum += add;
                }
            } else {
                for key in (0..pivot).rev() {
                    sum -= histograms[c].data().get_count(key as usize, i);

                    if sum < center {
                        p_out.push(key);
                        histograms[c].set_sum(sum, i);
                        break;
                    }
                }
            }
        }

        let x_clamp = (x + i as u32).clamp(0, output.info().width - 1);
        output.set_pixel(x_clamp, y, &p_out);

        set_pivots_med(histograms, &p_out, i);
    }
}

fn add_row_med(histograms: &mut Vec<MedianHist>, p_in: &Vec<&[u8]>, channels: usize) {
    for c in 0..channels {
        histograms[c].update(p_in, c, true);
    }
}

fn remove_row_med(histograms: &mut Vec<MedianHist>, p_in: &Vec<&[u8]>, channels: usize) {
    for c in 0..channels {
        histograms[c].update(p_in, c, false);
    }
}

fn set_pivots_med(histograms: &mut Vec<MedianHist>, pivots: &Vec<u8>, index: usize) {
    for c in 0..pivots.len() {
        histograms[c].set_pivot(pivots[c], index);
    }
}

////////////////////////////////////////
// Alpha-trimmed mean filter functions
////////////////////////////////////////

#[derive(Debug, Clone)]
struct MeanHist {
    data: PartialHistograms,
    sums: Vec<i32>,
    lower: Vec<Vec<u8>>,
    upper: Vec<Vec<u8>>,
    trim: usize,
    len: f32,
}

impl MeanHist {
    fn new(radius: usize, n_cols: usize, alpha: u32) -> Self {
        let size = 2 * radius + 1;
        let len = ((size * size) - alpha as usize) as f32;

        MeanHist {
            data: PartialHistograms::new(radius, n_cols),
            sums: Vec::with_capacity(n_cols),
            lower: Vec::with_capacity(n_cols),
            upper: Vec::with_capacity(n_cols),
            trim: (alpha as usize) / 2,
            len,
        }
    }

    fn data(&self) -> &PartialHistograms {
        &self.data
    }

    fn init(&mut self) {
        self.sums = vec![0; self.data.n_cols];
        self.lower = vec![Vec::with_capacity(self.trim); self.data.n_cols];
        self.upper = vec![Vec::with_capacity(self.trim); self.data.n_cols];
    }

    fn update(&mut self, p_in: &Vec<&[u8]>, channel_index: usize, add: bool) {
        if !self.sums.is_empty() {
            if add {
                for n in 0..self.data.n_cols {
                    for i in n..(n + self.data.size) {
                        let val = p_in[i][channel_index];
                        let lower = self.lower(n);
                        let upper = self.upper(n);

                        if val < lower {
                            self.lower[n].remove(self.trim -  1);
                            self.sums[n] += lower as i32;

                            let pos = self.lower[n].binary_search(&val).unwrap_or_else(|e| e);
                            self.lower[n].insert(pos, val);
                        } else if val > upper {
                            self.upper[n].remove(self.trim - 1);
                            self.sums[n] += upper as i32;

                            let pos = self.lower[n].binary_search_by_key(&Reverse(&val), Reverse).unwrap_or_else(|e| e);
                            self.upper[n].insert(pos, val);
                        } else {
                            self.sums[n] += val as i32;
                        }
                    }
                }
                self.data.update(p_in, channel_index, add);
            } else {
                self.data.update(p_in, channel_index, add);
                for n in 0..self.data.n_cols {
                    for i in n..(n + self.data.size) {
                        let val = p_in[i][channel_index];
                        let lower = self.lower(n);
                        let upper = self.upper(n);

                        let mut lower_count = self.data.get_count(lower as usize, n);
                        let mut upper_count = self.data.get_count(upper as usize, n);

                        for j in i..(n + self.data.size) {
                            if p_in[j][channel_index] == lower {
                                lower_count += 1;
                            } else if p_in[j][channel_index] == upper {
                                upper_count += 1;
                            }
                        }

                        for j in self.lower[n].iter().rev() {
                            if *j == lower {
                                lower_count -= 1;
                            } else {
                                break;
                            }
                        }

                        for j in self.upper[n].iter().rev() {
                            if *j == upper {
                                upper_count -= 1;
                            } else {
                                break;
                            }
                        }

                        if val == lower && lower_count == 0 {
                            self.lower[n].remove(self.trim - 1);
                            self.get_next_lower(n, lower_count, lower);
                        } else if val < lower {
                            let res = self.lower[n].binary_search(&val);

                            match res {
                                Ok(pos) => {
                                    self.lower[n].remove(pos);
                                    self.get_next_lower(n, lower_count, lower);
                                },
                                Err(_) => {
                                    self.sums[n] -= val as i32;
                                }
                            }
                        } else if val == upper && upper_count == 0 {
                            self.upper[n].remove(self.trim - 1);
                            self.get_next_upper(n, upper_count, upper);
                        } else if val > upper {
                            let res = self.lower[n].binary_search_by_key(&Reverse(&val), Reverse);

                            match res {
                                Ok(pos) => {
                                    self.upper[n].remove(pos);
                                    self.get_next_upper(n, upper_count, upper);
                                },
                                Err(_) => {
                                    self.sums[n] -= val as i32;
                                }
                            }
                        } else {
                            self.sums[n] -= val as i32;
                        }
                    }
                }
            }
        } else {
            self.data.update(p_in, channel_index, add);
        }
    }

    fn set_sum(&mut self, sum: i32, index: usize) {
        self.sums[index] = sum;
    }

    fn set_upper(&mut self, vals: Vec<u8>, index: usize) {
        self.upper[index] = vals;
    }

    fn set_lower(&mut self, vals: Vec<u8>, index: usize) {
        self.lower[index] = vals;
    }

    fn upper(&self, index: usize) -> u8 {
        self.upper[index][self.trim-1]
    }

    fn lower(&self, index: usize) -> u8 {
        self.lower[index][self.trim-1]
    }

    fn get_mean(&self, index: usize) -> u8 {
        ((self.sums[index] as f32) / self.len).round() as u8
    }

    fn get_next_lower(&mut self, n: usize, lower_count: i32, lower: u8) {
        if lower_count > 0 {
            self.lower[n].push(lower);
            self.sums[n] -= lower as i32;
        } else {
            for key in (lower + 1)..=255 {
                if self.data.get_count(key as usize, n) > 0 {
                    self.lower[n].push(key);
                    self.sums[n] -= key as i32;
                    break;
                }
            }
        }
    }

    fn get_next_upper(&mut self, n: usize, upper_count: i32, upper: u8) {
        if upper_count > 0 {
            self.upper[n].push(upper);
            self.sums[n] -= upper as i32;
        } else {
            for key in (0..upper).rev() {
                if self.data.get_count(key as usize, n) > 0 {
                    self.upper[n].push(key);
                    self.sums[n] -= key as i32;
                    break;
                }
            }
        }
    }
}

fn process_cols_mean(input: &Image<u8>, output: &mut Image<u8>, radius: u32, alpha: u32, n_cols: usize, x: u32) {
    let (width, height, channels) = input.info().whc();
    let mut histograms = vec![MeanHist::new(radius as usize, n_cols, alpha); channels as usize];

    // Initialize histogram and process first row
    init_cols_mean(input, output, &mut histograms, radius, alpha, n_cols, x);

    // Update histogram and process remaining rows
    for j in 1..height {
        // Update histograms
        let mut p_in = Vec::with_capacity(n_cols);
        let mut p_out = Vec::with_capacity(n_cols);
        let j_in = (j + radius).clamp(0, input.info().height - 1);
        let j_out = (j as i32 - radius as i32 - 1).clamp(0, input.info().height as i32 - 1) as u32;

        for i in (x as i32 - radius as i32)..((x + n_cols as u32 + radius) as i32) {
            let i_clamp = i.clamp(0, width as i32 - 1) as u32;
            p_in.push(input.get_pixel_unchecked(i_clamp, j_in));
            p_out.push(input.get_pixel_unchecked(i_clamp, j_out));
        }

        add_row_mean(&mut histograms, &p_in, channels as usize);
        remove_row_mean(&mut histograms, &p_out, channels as usize);

        process_row_mean(output, &mut histograms, n_cols, x, j);
    }
}

fn init_cols_mean(input: &Image<u8>, output: &mut Image<u8>, histograms: &mut Vec<MeanHist>, radius: u32, alpha: u32, n_cols: usize, x: u32) {
    let (width, height, channels) = input.info().whc();
    let size = 2 * radius + 1;

    // Initialize histograms
    for j in -(radius as i32)..(radius as i32 + 1) {
        let mut p_in = Vec::with_capacity(n_cols);
        for i in (x as i32 - radius as i32)..((x + n_cols as u32 + radius) as i32) {
            p_in.push(input.get_pixel_unchecked(i.clamp(0, width as i32 - 1) as u32,
                                                j.clamp(0, height as i32 - 1) as u32));
        }

        add_row_mean(histograms, &p_in, channels as usize);
    }

    // Initialize histograms
    for c in 0..(channels as usize) {
        histograms[c].init();
    }

    // Compute first mean values
    let trim = (alpha as usize) / 2;
    let upper_trim = (size * size) as usize - trim;
    for i in 0..n_cols {
        let mut p_out = Vec::with_capacity(channels as usize);
        for c in 0..(channels as usize) {
            let mut count = 0;
            let mut sum = 0;
            let mut upper = Vec::with_capacity(trim);
            let mut lower = Vec::with_capacity(trim);

            for key in 0u8..=255 {
                let mut add = histograms[c].data().get_count(key as usize, i);
                count += add;
                sum += add * key as i32;

                while lower.len() < trim && add > 0 {
                    lower.push(key);
                    sum -= key as i32;
                    add -= 1;
                }

                while (count as usize) > upper_trim && upper.len() < trim && add > 0 {
                    upper.insert(0, key);
                    sum -= key as i32;
                    add -= 1;
                }
            }

            histograms[c].set_sum(sum, i);
            histograms[c].set_upper(upper, i);
            histograms[c].set_lower(lower, i);

            p_out.push(histograms[c].get_mean(i));
        }

        let x_clamp = (x + i as u32).clamp(0, output.info().width - 1);
        output.set_pixel(x_clamp, 0, &p_out);
    }
}

fn process_row_mean(output: &mut Image<u8>, histograms: &mut Vec<MeanHist>, n_cols: usize, x: u32, y: u32) {
    let channels = output.info().channels as usize;

    for i in 0..n_cols {
        let mut p_out = Vec::with_capacity(channels);
        for c in 0..channels {
            p_out.push(histograms[c].get_mean(i));
        }

        let x_clamp = (x + i as u32).clamp(0, output.info().width - 1);
        output.set_pixel(x_clamp, y, &p_out);
    }
}

fn add_row_mean(histograms: &mut Vec<MeanHist>, p_in: &Vec<&[u8]>, channels: usize) {
    for c in 0..channels {
        histograms[c].update(p_in, c, true);
    }
}

fn remove_row_mean(histograms: &mut Vec<MeanHist>, p_in: &Vec<&[u8]>, channels: usize) {
    for c in 0..channels {
        histograms[c].update(p_in, c, false);
    }
}