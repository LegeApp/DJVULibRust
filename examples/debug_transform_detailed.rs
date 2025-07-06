// examples/debug_transform_detailed.rs
// Detailed analysis of the wavelet transform on 4x4 solid color

use djvu_encoder::encode::iw44::transform::{Encode, mirror};

fn print_4x4_data(data: &[i32], label: &str) {
    println!("{}", label);
    for y in 0..4 {
        let row: Vec<i32> = (0..4).map(|x| data[y * 4 + x]).collect();
        println!("  row {}: {:?}", y, row);
    }
}

fn test_mirror_function() {
    println!("=== Testing Mirror Function ===");
    for i in -5..10 {
        let mirrored = mirror(i, 4);
        println!("mirror({}, 4) = {}", i, mirrored);
    }
}

fn test_manual_lifting_step() {
    println!("\n=== Manual 4x4 Solid Color Lifting ===");
    
    // Create 4x4 solid color data
    let value = -60i32 << 6; // Apply IW_SHIFT
    let mut data = vec![value; 16];
    
    println!("Input (all values = {}):", value);
    print_4x4_data(&data, "Original:");
    
    // Apply a single scale (scale=1) manually to understand what's happening
    
    // 1. Vertical filter first (filter_fv equivalent)
    println!("\n--- Vertical Filter (Scale 1) ---");
    let w = 4;
    let h = 4;
    let rowsize = 4;
    let scale = 1;
    let s = scale * rowsize; // s = 4
    let s3 = 3 * s; // s3 = 12
    
    // For scale=1, we process every row (y = 0, 1, 2, 3)
    for y in 0..h {
        let p_idx = y * rowsize;
        
        // Lifting step: d[n] = x_o[n] - (-x_e[n-1] + 9*x_e[n] + 9*x_e[n+1] - x_e[n+2]) / 16
        // But in IW44, the formula is slightly different due to the lifting implementation
        
        println!("Row {} (p_idx = {}):", y, p_idx);
        
        for x in 0..w {
            let i = p_idx + x;
            
            // Calculate neighbors using boundary conditions
            let val_minus_s = if y >= 1 { data[i - s] } else { data[i] }; // symmetric boundary
            let val_plus_s = if y + 1 < h { data[i + s] } else { data[i] }; // symmetric boundary
            let val_minus_s3 = if y >= 3 { data[i - s3] } else { data[i] }; // symmetric boundary  
            let val_plus_s3 = if y + 3 < h { data[i + s3] } else { data[i] }; // symmetric boundary
            
            let a = val_minus_s as i32 + val_plus_s as i32;
            let b = val_minus_s3 as i32 + val_plus_s3 as i32;
            let delta = (((a * 9) - b + 16) >> 5);
            let new_val = data[i] as i32 - delta;
            
            println!("  x={}: neighbors=({},{},{},{}) a={} b={} delta={} {} -> {}", 
                     x, val_minus_s3, val_minus_s, val_plus_s, val_plus_s3,
                     a, b, delta, data[i], new_val);
                     
            data[i] = new_val as i32;
        }
    }
    
    print_4x4_data(&data, "After vertical filter:");
    
    // 2. Horizontal filter (filter_fh equivalent)
    println!("\n--- Horizontal Filter (Scale 1) ---");
    
    // For each row, apply horizontal lifting
    for y in 0..h {
        let row_start = y * rowsize;
        let row = &mut data[row_start..row_start + w];
        
        println!("Processing row {}: {:?}", y, row);
        
        // This is a simplified version of what should happen
        // The actual filter_fh is more complex, but for solid color
        // we can analyze what should happen
        
        let s = scale; // s = 1  
        let s2 = 2 * s; // s2 = 2
        let s3 = 3 * s; // s3 = 3
        
        // For a 4-element row with scale=1:
        // Even positions: 0, 2 (s2=2)
        // Odd positions: 1, 3
        
        let original_row = row.to_vec();
        
        // Calculate new values for even positions (0, 2)
        for i in 0..((w + s2 - 1) / s2) { // i = 0, 1 for positions 0, 2
            let x = i * s2; // x = 0, 2
            
            if x < w {
                // Get neighbors at +/-s and +/-s3 positions
                let idx_minus_s3 = mirror(x as isize - s3 as isize, w);
                let idx_minus_s = mirror(x as isize - s as isize, w);
                let idx_plus_s = mirror(x as isize + s as isize, w);
                let idx_plus_s3 = mirror(x as isize + s3 as isize, w);
                
                let neighbors_1s = original_row[idx_minus_s] as i32 + original_row[idx_plus_s] as i32;
                let neighbors_3s = original_row[idx_minus_s3] as i32 + original_row[idx_plus_s3] as i32;
                
                let delta = (((neighbors_1s * 9) - neighbors_3s + 16) >> 5) as i32;
                let new_val = original_row[x].wrapping_sub(delta);
                
                println!("  Even pos {}: neighbors=({},{},{},{}) delta={} {} -> {}", 
                         x, original_row[idx_minus_s3], original_row[idx_minus_s], 
                         original_row[idx_plus_s], original_row[idx_plus_s3],
                         delta, original_row[x], new_val);
                
                row[x] = new_val;
            }
        }
        
        println!("  After horizontal lifting: {:?}", row);
    }
    
    print_4x4_data(&data, "After horizontal filter:");
    
    // Analyze the result
    let dc_coeff = data[0];
    let mut nonzero_count = 0;
    for &coeff in &data {
        if coeff != 0 {
            nonzero_count += 1;
        }
    }
    
    println!("\n=== Analysis ===");
    println!("DC coefficient: {}", dc_coeff);
    println!("Non-zero coefficients: {}/16", nonzero_count);
    println!("Expected for solid color: DC should be ~{}, all AC should be 0", value);
    
    if nonzero_count == 1 && dc_coeff != 0 {
        println!("✓ GOOD: Only DC coefficient is non-zero");
    } else {
        println!("✗ BAD: {} non-zero coefficients indicates transform bug", nonzero_count);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Detailed Transform Debug ===\n");
    
    test_mirror_function();
    test_manual_lifting_step();
    
    println!("\n=== Testing with Real Transform ===");
    
    // Test with the actual transform function
    let mut data = vec![-60i32 << 6; 16]; // Apply IW_SHIFT
    println!("Before transform: {:?}", data);
    
    Encode::forward::<4>(&mut data, 4, 4, 1); // 1 level, 4 SIMD lanes
    
    print_4x4_data(&data, "After real transform:");
    
    Ok(())
}
