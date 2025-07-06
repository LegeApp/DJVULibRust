// Debug the update step specifically for solid color images

use djvu_encoder::encode::iw44::transform::{mirror, fwt_horizontal_inplace, fwt_vertical_inplace};

fn test_simple_2x2() {
    println!("=== Testing 2x2 solid color 100 ===");
    
    // Create 2x2 solid color (should remain constant after transform)
    let mut data = [100i32, 100, 100, 100];  // DC coefficient of 100
    let w = 2;
    let h = 2;
    
    println!("Before transform: {:?}", data);
    
    // Apply 1 level of horizontal transform
    fwt_horizontal_inplace::<4>(&mut data, w, h, 1);
    println!("After horizontal: {:?}", data);
    
    // Apply 1 level of vertical transform  
    fwt_vertical_inplace::<4>(&mut data, w, h, 1);
    println!("After vertical: {:?}", data);
    
    // For solid color, we expect:
    // - DC coefficient (0,0) = original value
    // - All other coefficients = 0
    let dc = data[0];
    let ac_coeffs = &data[1..];
    let non_zero_acs = ac_coeffs.iter().filter(|&&x| x != 0).count();
    
    println!("Final DC: {}, Non-zero ACs: {}", dc, non_zero_acs);
    if dc == 100 && non_zero_acs == 0 {
        println!("✅ PERFECT");
    } else {
        println!("❌ IMPERFECT");
    }
}

fn test_simple_4x4() {
    println!("\n=== Testing 4x4 solid color 100 ===");
    
    // Create 4x4 solid color
    let mut data = vec![100i32; 16];
    let w = 4;
    let h = 4;
    
    println!("Before transform: DC at (0,0) = {}", data[0]);
    
    // Apply 2 levels of transform (4x4 can handle 2 levels)
    fwt_horizontal_inplace::<4>(&mut data, w, h, 2);
    println!("After horizontal: DC = {}", data[0]);
    
    fwt_vertical_inplace::<4>(&mut data, w, h, 2);
    println!("After vertical: DC = {}", data[0]);
    
    // Check results
    let dc = data[0];
    let non_zero_acs = data[1..].iter().filter(|&&x| x != 0).count();
    
    println!("Final DC: {}, Non-zero ACs: {}", dc, non_zero_acs);
    if dc == 100 && non_zero_acs == 0 {
        println!("✅ PERFECT");
    } else {
        println!("❌ IMPERFECT");
        // Show some AC values
        for (i, &coeff) in data.iter().enumerate() {
            if i > 0 && coeff != 0 {
                println!("  AC[{}] = {}", i, coeff);
            }
        }
    }
}

fn test_mirror_function() {
    println!("\n=== Testing mirror function ===");
    
    // Test the mirror function with small arrays
    for len in [2, 4, 8] {
        println!("Length {}: ", len);
        for idx in -8..=16 {
            let mirrored = mirror(idx, len);
            print!("{} -> {}, ", idx, mirrored);
        }
        println!();
    }
}

fn main() {
    test_mirror_function();
    test_simple_2x2();
    test_simple_4x4();
}
