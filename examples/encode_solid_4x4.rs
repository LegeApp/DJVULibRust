// examples/encode_solid_4x4.rs
// Debug a 4x4 solid color image through the IW44 encoder pipeline

use djvu_encoder::encode::iw44::{encoder::*, transform::Encode};
use image::{RgbImage, Rgb};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== IW44 Encoder Debug: 4x4 Solid Color ===\n");

    // Create a 4x4 solid color RGB image (blue)
    let width = 4;
    let height = 4;
    let solid_color = Rgb([50u8, 50u8, 200u8]); // Blue-ish
    
    let pixels: Vec<u8> = (0..width * height)
        .flat_map(|_| solid_color.0)
        .collect();
    
    let img = RgbImage::from_raw(width as u32, height as u32, pixels)
        .expect("Failed to create test image");

    println!("Created {}x{} image with solid color {:?}", width, height, solid_color);

    // Create encoder parameters
    let params = EncoderParams {
        decibels: None,
        crcb_mode: CrcbMode::None, // Keep it simple, no chrominance
        db_frac: 0.9,
    };

    println!("Creating IW44 encoder...");

    // Create encoder - this will trigger color conversion and transform
    let mut encoder = IWEncoder::from_rgb(&img, None, params)?;

    println!("Encoder created successfully. Now encoding first chunk...");

    // Encode the first chunk
    let result = encoder.encode_chunk(1)?;
    let (chunk_data, has_more) = result;

    println!("\n=== Encoding Results ===");
    println!("Chunk size: {} bytes", chunk_data.len());
    println!("Has more chunks: {}", has_more);
    println!("Compression ratio: {:.2}:1", 
        (width * height * 3) as f64 / chunk_data.len() as f64);

    // Analyze the chunk data
    if chunk_data.len() > 20 {
        println!("First 20 bytes: {:?}", &chunk_data[0..20]);
        println!("Last 10 bytes: {:?}", &chunk_data[chunk_data.len()-10..]);
    } else {
        println!("Full chunk: {:?}", chunk_data);
    }

    // Let's also test the transform directly on the solid color data
    println!("\n=== Direct Transform Test ===");
    
    // Convert RGB to YUV manually to see what the encoder sees
    let y_values: Vec<i32> = (0..height)
        .flat_map(|y| (0..width).map(move |x| {
            let pixel_idx = (y * width + x) * 3;
            let r = solid_color.0[0] as f64;
            let g = solid_color.0[1] as f64;
            let b = solid_color.0[2] as f64;
            
            // Standard RGB to YUV conversion
            let y_val = 0.299 * r + 0.587 * g + 0.114 * b;
            // Center around 0 and apply IW_SHIFT (left shift by 6)
            ((y_val - 128.0) as i32) << 6
        }))
        .collect();

    println!("Y channel values (first 16): {:?}", &y_values[0..y_values.len().min(16)]);
    println!("Expected Y value: {}", y_values[0]);
    
    // Test transform directly
    let mut transform_data = y_values.clone();
    println!("Before transform: all values = {}", transform_data[0]);
    
    // Apply transform using new API with correct level calculation
    let max_levels = ((width.min(height) as f64).log2().floor() as usize).max(1);
    println!("Using {} decomposition levels", max_levels);
    Encode::forward::<4>(&mut transform_data, width, height, max_levels);
    
    println!("After transform:");
    println!("  DC coefficient: {}", transform_data[0]);
    println!("  All coefficients: {:?}", transform_data);
    
    // Count non-zero AC coefficients
    let non_zero_ac = transform_data[1..].iter().filter(|&&x| x != 0).count();
    let max_ac = transform_data[1..].iter().map(|&x| x.abs()).max().unwrap_or(0);
    
    println!("  Non-zero AC coefficients: {}/{}", non_zero_ac, transform_data.len() - 1);
    println!("  Max AC magnitude: {}", max_ac);
    
    // Expected behavior: for solid color, only DC should be non-zero
    if non_zero_ac == 0 {
        println!("✓ GOOD: Transform correctly produces only DC coefficient for solid color");
    } else {
        println!("✗ BAD: Transform incorrectly produces {} non-zero AC coefficients", non_zero_ac);
        println!("      This indicates a bug in the wavelet transform implementation");
    }

    println!("\n=== Summary ===");
    println!("Input: {}x{} solid color image", width, height);
    println!("Encoded size: {} bytes", chunk_data.len());
    println!("Transform AC coefficients: {} (should be 0)", non_zero_ac);
    
    if non_zero_ac > 0 && chunk_data.len() > 50 {
        println!("DIAGNOSIS: Large file size + non-zero AC coefficients suggests transform bug");
    } else if non_zero_ac == 0 && chunk_data.len() > 50 {
        println!("DIAGNOSIS: Good transform but large file suggests codec issue");
    } else if non_zero_ac == 0 && chunk_data.len() <= 50 {
        println!("DIAGNOSIS: Everything looks good - efficient compression achieved");
    }

    Ok(())
}
