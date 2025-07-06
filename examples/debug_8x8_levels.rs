// Debug the 8x8 case step by step

use djvu_encoder::encode::iw44::transform::{mirror, fwt_horizontal_inplace, fwt_vertical_inplace};

fn test_8x8_step_by_step() {
    println!("=== Testing 8x8 solid color 100 step by step ===");
    
    // Create 8x8 solid color
    let mut data = vec![100i32; 64];
    let w = 8;
    let h = 8;
    
    println!("Before transform: DC = {}", data[0]);
    println!("Sample values: {:?}", &data[0..8]);
    
    // Apply each level and each pass separately
    for level in 1..=3 {
        println!("\n--- Level {} ---", level);
        
        // Horizontal pass
        let mut data_h = data.clone();
        fwt_horizontal_inplace::<4>(&mut data_h, w, h, level);
        println!("After {} horizontal levels: DC = {}", level, data_h[0]);
        let h_non_zero = data_h[1..].iter().filter(|&&x| x != 0).count();
        println!("  Non-zero ACs after horizontal: {}", h_non_zero);
        
        // Vertical pass
        let mut data_v = data_h.clone();
        fwt_vertical_inplace::<4>(&mut data_v, w, h, level);
        println!("After {} vertical levels: DC = {}", level, data_v[0]);
        let v_non_zero = data_v[1..].iter().filter(|&&x| x != 0).count();
        println!("  Non-zero ACs after vertical: {}", v_non_zero);
        
        if v_non_zero > 0 {
            println!("  First few non-zero ACs:");
            for (i, &coeff) in data_v.iter().enumerate() {
                if i > 0 && coeff != 0 {
                    println!("    AC[{}] = {}", i, coeff);
                    if i > 10 { break; } // Don't flood output
                }
            }
        }
        
        data = data_v; // Use this for next level
    }
}

fn test_level_by_level() {
    println!("\n=== Testing each decomposition level individually ===");
    
    for max_levels in 1..=3 {
        println!("\n--- {} levels total ---", max_levels);
        let mut data = vec![100i32; 64];
        
        fwt_horizontal_inplace::<4>(&mut data, 8, 8, max_levels);
        fwt_vertical_inplace::<4>(&mut data, 8, 8, max_levels);
        
        let dc = data[0];
        let non_zero_acs = data[1..].iter().filter(|&&x| x != 0).count();
        println!("Final DC: {}, Non-zero ACs: {}", dc, non_zero_acs);
        
        if dc != 100 || non_zero_acs > 0 {
            println!("❌ Issues detected at {} levels", max_levels);
            
            // Show problematic coefficients
            for (i, &coeff) in data.iter().enumerate() {
                if (i == 0 && coeff != 100) || (i > 0 && coeff != 0) {
                    println!("  coeff[{}] = {}", i, coeff);
                }
            }
        } else {
            println!("✅ Perfect at {} levels", max_levels);
        }
    }
}

fn main() {
    test_8x8_step_by_step();
    test_level_by_level();
}
