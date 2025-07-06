use std::simd::{LaneCount, Simd, SupportedLaneCount};

use super::constants::IW_SHIFT;
use ::image::GrayImage;

// Use a type alias for Simd<i16, 4>
type I16x4 = Simd<i16, 4>;

/// Saturating conversion from i32 to i16 to prevent overflow
#[inline]
fn sat16(x: i32) -> i16 {
    if x > 32_767 { 
        32_767 
    } else if x < -32_768 { 
        -32_768 
    } else { 
        x as i16 
    }
}

pub struct Encode;

impl Encode {
    /// Forward wavelet transform using the lifting scheme.
    /// Port of `IW44Image::Transform::Encode::forward` from DjVuLibre.
    /// Uses vertical then horizontal filtering per scale, as per DjVu specification.
    /// This order is critical for preserving DC coefficients in solid color images.
    ///
    /// # Arguments
    /// * `buf` - Coefficient data to transform (modified in-place)
    /// * `w` - Image width
    /// * `h` - Image height
    /// * `levels` - Number of decomposition levels
    pub fn forward<const LANES: usize>(
        buf: &mut [i32],
        w: usize,
        h: usize,
        levels: usize,
    ) where
        LaneCount<LANES>: SupportedLaneCount,
    {
        // Work on progressively smaller low-pass rectangles
        let mut cur_w = w;
        let mut cur_h = h;

        for level in 0..levels {
            let scale = 1 << level;  // Scale factor for this level
            
            // Vertical, then horizontal, on the top-left (low-pass) area only
            // This order is critical to preserve DC coefficients exactly
            fwt_vertical_inplace_single_level::<LANES>(buf, w, cur_w, cur_h, scale);
            fwt_horizontal_inplace_single_level::<LANES>(buf, w, cur_w, cur_h, scale);

            // Next scale works on the even samples only
            cur_w = (cur_w + 1) / 2;
            cur_h = (cur_h + 1) / 2;
        }
    }
    
    /// Prepare image data for wavelet transform with proper pixel shifting and centering.
    /// This handles the conversion from various input formats to the i32 buffer format expected by the transform.
    pub fn prepare_and_transform<F>(
        data32: &mut [i32], 
        w: usize, 
        h: usize, 
        pixel_fn: F
    ) where F: Fn(usize, usize) -> i32 
    {
        // Copy pixels with proper shifting
        for y in 0..h {
            for x in 0..w {
                data32[y * w + x] = pixel_fn(x, y);
            }
        }
        
        // Debug: Check input data before transform
        let sample_indices = [0, w/4, w/2, 3*w/4, w-1];
        let mut input_samples = Vec::new();
        let mut min_val = i32::MAX;
        let mut max_val = i32::MIN;
        let mut unique_vals = std::collections::HashSet::new();
        
        for y in 0..std::cmp::min(h, 5) { // Check first 5 rows
            for x in &sample_indices {
                if *x < w {
                    let val = data32[y * w + *x];
                    input_samples.push(val);
                    min_val = min_val.min(val);
                    max_val = max_val.max(val);
                    if unique_vals.len() < 10 {
                        unique_vals.insert(val);
                    }
                }
            }
        }
        
        println!("DEBUG Transform input (after {} shift):", IW_SHIFT);
        println!("  Input range: {} to {} (span: {})", min_val, max_val, max_val - min_val);
        println!("  Unique values: {:?}", unique_vals);
        println!("  Sample values: {:?}", input_samples);
        
        // Apply forward transform with appropriate number of levels
        // For WxH image, max levels = floor(log2(min(W, H)))
        let max_levels = ((w.min(h) as f64).log2().floor() as usize).max(1);
        Self::forward::<4>(data32, w, h, max_levels);
        
        // Debug: Check coefficients after transform
        let mut coeff_min = i32::MAX;
        let mut coeff_max = i32::MIN;
        let mut coeff_samples = Vec::new();
        let mut zero_count = 0;
        let mut nonzero_count = 0;
        
        for y in 0..std::cmp::min(h, 5) { // Check first 5 rows
            for x in &sample_indices {
                if *x < w {
                    let coeff = data32[y * w + *x];
                    coeff_samples.push(coeff);
                    coeff_min = coeff_min.min(coeff);
                    coeff_max = coeff_max.max(coeff);
                    if coeff == 0 {
                        zero_count += 1;
                    } else {
                        nonzero_count += 1;
                    }
                }
            }
        }
        
        // Check DC coefficient (should be average of image for solid color)
        let dc_coeff = data32[0];
        
        println!("DEBUG Transform output:");
        println!("  DC coefficient (0,0): {}", dc_coeff);
        println!("  Coeff range: {} to {} (span: {})", coeff_min, coeff_max, coeff_max as i32 - coeff_min as i32);
        println!("  Sample coefficients: {:?}", coeff_samples);
        println!("  Zero coeffs: {}, Nonzero coeffs: {}", zero_count, nonzero_count);
        
        // For solid color, we expect mostly zeros except for DC
        if zero_count > nonzero_count * 10 {
            println!("  *** SPARSE COEFFICIENTS - good for compression! ***");
        } else {
            println!("  *** WARNING: Too many non-zero coefficients for solid color! ***");
        }
    }
    
    /// Helper for unsigned 8-bit image data (like GrayImage)
    pub fn from_u8_image(img: &GrayImage, data32: &mut [i32], w: usize, h: usize) {
        Self::prepare_and_transform(data32, w, h, |x, y| {
            let pixel_u8 = img.get_pixel(x as u32, y as u32)[0] as i32;
            pixel_u8 << IW_SHIFT
        });
    }
    
    /// Helper for signed 8-bit channel data (Y, Cb, Cr)
    pub fn from_i8_channel(channel_buf: &[i8], data32: &mut [i32], w: usize, h: usize) {
        let img_w = if channel_buf.len() >= w * h { w } else { 
            // For smaller images, compute actual width
            let pixels = channel_buf.len();
            if pixels > 0 && h > 0 { 
                std::cmp::min(w, pixels / h)
            } else { 
                w 
            }
        };
        let img_h = if channel_buf.len() >= w * h { h } else {
            // For smaller images, compute actual height
            let pixels = channel_buf.len();
            if pixels > 0 && img_w > 0 { 
                std::cmp::min(h, pixels / img_w)
            } else { 
                h 
            }
        };
        
        Self::prepare_and_transform(data32, w, h, |x, y| {
            // Use mirroring for coordinates outside the actual image bounds
            let mirror_x = if x >= img_w {
                mirror(x as isize, img_w)
            } else {
                x
            };
            let mirror_y = if y >= img_h {
                mirror(y as isize, img_h)
            } else {
                y
            };
            
            let idx = mirror_y * img_w + mirror_x;
            if idx < channel_buf.len() {
                let pixel_i8 = channel_buf[idx];
                (pixel_i8 as i32) << IW_SHIFT
            } else {
                // Fallback: use last pixel value
                if channel_buf.len() > 0 {
                    (channel_buf[channel_buf.len() - 1] as i32) << IW_SHIFT
                } else {
                    0
                }
            }
        });
    }
}

/// Mirror index for boundaries: even symmetry around 0 and around size-1
/// Ported from DjVuLibre.
#[inline]
fn mirror(mut k: isize, size: usize) -> usize {
    if size == 0 {
        return 0;
    }
    let size = size as isize;
    if k < 0 {
        k = -k;
    }
    let period = (size - 1) * 2;
    k %= period;
    if k >= size {
        k = period - k;
    }
    k as usize
}

/// Convenience wrapper: *horizontal → vertical* forward IW-44 transform.
///
/// Call this instead of the two passes if you just want the full 2-D
/// decomposition in one line.
///
/// # Example
/// ```rust
/// fwt_forward_inplace::<8>(&mut image, w, h, /*levels=*/5);
/// quantise_inplace(&mut image, w, h, quant_table);
/// ```

/// In-place *vertical* pass for **one** decomposition level.
///
/// * `buf`  – pixel buffer (row-major).
/// * `full_w` – full image width (for row addressing).
/// * `work_w/work_h` – working rectangle dimensions to process.
/// * `scale` – current scale (1 << level).
///
/// Processes only the current level at the given scale.
pub fn fwt_vertical_inplace_single_level<const LANES: usize>(
    buf: &mut [i32],
    full_w: usize,
    work_w: usize,
    work_h: usize,
    scale: usize,
) where
    LaneCount<LANES>: SupportedLaneCount,
{
    // If the working height is smaller than the scale, the transform is a no-op.
    if work_h <= scale {
        return;
    }

    assert!(
        buf.len() >= work_h * full_w,
        "buffer length must be at least work_h * full_w"
    );

    // Process columns sequentially within the working rectangle
    for col in 0..work_w {
        // Create a vector to hold this column's data
        let mut col_vec = vec![0i32; work_h];
        
        // Gather column data
        for y in 0..work_h {
            col_vec[y] = buf[y * full_w + col];
        }

        // Re-usable detail buffer (predict output).
        let mut detail = vec![0i32; work_h];

        let s = scale;  // current scale
        let dbl = s * 2; // stride between low-pass samples

        /* ---------- PREDICT (1-Δ) ---------- */
        let mut y = s;
        let simd_step = dbl * LANES; // each SIMD lane processes an odd sample

        // SIMD core
        while y + simd_step <= work_h {
            let mut centre = [0i32; LANES];
            let mut above  = [0i32; LANES];
            let mut below  = [0i32; LANES];

            for lane in 0..LANES {
                let yi = y + lane * dbl;
                centre[lane] = col_vec[yi];
                above[lane]  = col_vec[mirror(yi as isize - s as isize, work_h)];
                below[lane]  = col_vec[mirror(yi as isize + s as isize, work_h)];
            }

            let c = Simd::<i32, LANES>::from_array(centre);
            let a = Simd::<i32, LANES>::from_array(above);
            let b = Simd::<i32, LANES>::from_array(below);

            let pred = (a + b) >> Simd::splat(1); // floor((a+b)/2)
            let det = c - pred;

            for lane in 0..LANES {
                detail[y + lane * dbl] = det[lane];
            }
            y += simd_step;
        }

        // Scalar tail / boundaries
        while y < work_h {
            let ya = mirror(y as isize - s as isize, work_h);
            let yb = mirror(y as isize + s as isize, work_h);
            let pred = (col_vec[ya] + col_vec[yb]) >> 1; // floor division
            detail[y] = col_vec[y] - pred;
            y += dbl;
        }

        // ---- UPDATE STEP ----
        let mut k = 0;
        while k < work_h {
            let left_idx = mirror(k as isize - s as isize, work_h);
            let right_idx = mirror(k as isize + s as isize, work_h);
            let left_detail = detail[left_idx];
            let right_detail = detail[right_idx];
            
            let sum = left_detail + right_detail;
            let update = sum >> 1;
            
            col_vec[k] += update;
            k += dbl;
        }

        /* ---------- COPY DETAIL BACK ---------- */
        let mut y_copy = s;
        while y_copy < work_h {
            col_vec[y_copy] = detail[y_copy];
            y_copy += dbl;
        }

        // Scatter column data back
        for y_scatter in 0..work_h {
            buf[y_scatter * full_w + col] = col_vec[y_scatter];
        }
    }
}

/// In-place *horizontal* pass for **one** decomposition level.
///
/// * `buf`   – image plane, row-major, mutable so we can overwrite in place.
/// * `full_w` – full image width (for row addressing).
/// * `work_w/work_h` – working rectangle dimensions to process.
/// * `scale` – current scale (1 << level).
///
/// Processes only the current level at the given scale.
pub fn fwt_horizontal_inplace_single_level<const LANES: usize>(
    buf: &mut [i32],
    full_w: usize,
    work_w: usize,
    work_h: usize,
    scale: usize,
) where
    LaneCount<LANES>: SupportedLaneCount,
{
    // No-op when width too small
    if work_w <= scale { return; }
    assert!(buf.len() >= work_h * full_w, "buffer must be at least work_h * full_w pixels");

    let s = scale;
    let dbl = s * 2;

    for row in 0..work_h {
        let row_start = row * full_w;
        let row_slice = &mut buf[row_start..row_start + work_w];
        let mut detail = vec![0i32; work_w];

        // PREDICT step: compute detail coefficients
        let mut k = s;
        while k < work_w {
            let c = row_slice[k];
            let k0 = k as isize;
            let c_m3 = if k0 >= 3 * (s as isize) { row_slice[((k0 - 3 * (s as isize)) as usize)] } else { 0 };
            let c_m1 = if k0 >= (s as isize) { row_slice[((k0 - (s as isize)) as usize)] } else { 0 };
            let c_p1 = if (k0 + (s as isize)) < (work_w as isize) { row_slice[((k0 + (s as isize)) as usize)] } else { 0 };
            let c_p3 = if (k0 + 3 * (s as isize)) < (work_w as isize) { row_slice[((k0 + 3 * (s as isize)) as usize)] } else { 0 };
            let pred = (-c_m3 + 9 * c_m1 + 9 * c_p1 - c_p3) >> 4;
            detail[k] = c - pred;
            k += dbl;
        }

        // UPDATE step: adjust approximation coefficients
        let mut k = 0;
        while k < work_w {
            let k0 = k as isize;
            let d_m3 = if k0 >= 3 * (s as isize) { detail[((k0 - 3 * (s as isize)) as usize)] } else { 0 };
            let d_m1 = if k0 >= (s as isize) { detail[((k0 - (s as isize)) as usize)] } else { 0 };
            let d_p1 = if (k0 + (s as isize)) < (work_w as isize) { detail[((k0 + (s as isize)) as usize)] } else { 0 };
            let d_p3 = if (k0 + 3 * (s as isize)) < (work_w as isize) { detail[((k0 + 3 * (s as isize)) as usize)] } else { 0 };
            let update = (-d_m3 + 9 * d_m1 + 9 * d_p1 - d_p3) >> 4;
            row_slice[k] += update;
            k += dbl;
        }

        // WRITE detail coefficients back
        let mut k = s;
        while k < work_w {
            row_slice[k] = detail[k];
            k += dbl;
        }
    }
}
