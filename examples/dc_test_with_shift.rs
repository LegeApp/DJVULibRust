use djvu_encoder::encode::iw44::transform::Encode;
use djvu_encoder::encode::iw44::constants::IW_SHIFT;

fn test_perfect_dc_preservation_with_shift(value: i32, size: usize, description: &str) -> (bool, bool, i32, i32) {
    println!("\n=== Testing {}: {}x{} solid color {} ===", description, size, size, value);
    
    // Apply IW_SHIFT like the real encoder does
    let shifted_value = value << IW_SHIFT;
    let mut data = vec![shifted_value; size * size];
    let original_dc = shifted_value;
    
    println!("Original value: {} -> shifted: {} (shift: {})", value, shifted_value, IW_SHIFT);
    
    // Apply transform with maximum levels for this size
    let max_levels = (size as f32).log2() as usize;
    println!("Using {} levels for {}x{} image", max_levels, size, size);
    
    Encode::forward::<1>(&mut data, size, size, max_levels);
    
    let result_dc = data[0];
    let dc_error = result_dc - original_dc;
    
    // Count non-zero AC coefficients
    let mut ac_count = 0;
    let mut max_ac = 0i32;
    let mut ac_samples = Vec::new();
    
    for i in 1..data.len() {
        if data[i] != 0 {
            ac_count += 1;
            max_ac = max_ac.max(data[i].abs());
            if ac_samples.len() < 10 {  // Show first 10 non-zero ACs
                ac_samples.push((i, data[i]));
            }
        }
    }
    
    println!("Results:");
    println!("  Original value: {}", value);
    println!("  Shifted input DC: {}", original_dc);
    println!("  Transform DC: {}", result_dc);
    println!("  DC error: {} ({}%)", dc_error, if original_dc != 0 { (dc_error as f64 / original_dc as f64 * 100.0) } else { 0.0 });
    println!("  Expected DC after shift-back: {}", result_dc >> IW_SHIFT);
    println!("  Non-zero ACs: {}/{}", ac_count, data.len() - 1);
    println!("  Max AC magnitude: {}", max_ac);
    if !ac_samples.is_empty() {
        println!("  Sample ACs: {:?}", ac_samples);
    }
    
    // For perfect solid color, we expect:
    // 1. DC should be exactly preserved (at the shifted level)
    // 2. All AC coefficients should be exactly zero
    let perfect_dc = dc_error == 0;
    let perfect_acs = ac_count == 0;
    
    if perfect_dc && perfect_acs {
        println!("  ✅ PERFECT: DC preserved exactly, all ACs zero");
    } else {
        println!("  ❌ IMPERFECT:");
        if !perfect_dc {
            println!("     - DC drift: {} -> {} (error: {})", original_dc, result_dc, dc_error);
        }
        if !perfect_acs {
            println!("     - {} non-zero AC coefficients (max: {})", ac_count, max_ac);
        }
    }
    
    (perfect_dc, perfect_acs, dc_error, ac_count)
}

fn main() {
    println!("=== DC Preservation Test with Proper IW_SHIFT ===");
    println!("IW_SHIFT = {} (multiply by {})", IW_SHIFT, 1 << IW_SHIFT);
    
    let mut total_tests = 0;
    let mut perfect_tests = 0;
    let mut dc_perfect_tests = 0;
    let mut ac_perfect_tests = 0;
    
    // Test various combinations with the problem cases
    let test_cases = [
        (100, 4, "Small positive"),
        (-100, 4, "Small negative"),
        (0, 4, "Zero 4x4"),
        (100, 8, "Medium positive"),
        (-100, 8, "Medium negative"),
        (-448, 8, "Problem case (-448)"), // The original problematic case
        (100, 16, "Large positive"),
        (-100, 16, "Large negative"),
        (256, 8, "Power of 2"),
        (-256, 8, "Negative power of 2"),
    ];
    
    for (value, size, desc) in &test_cases {
        let (dc_perfect, ac_perfect, dc_error, ac_count) = test_perfect_dc_preservation_with_shift(*value, *size, desc);
        
        total_tests += 1;
        if dc_perfect { dc_perfect_tests += 1; }
        if ac_perfect { ac_perfect_tests += 1; }
        if dc_perfect && ac_perfect { perfect_tests += 1; }
    }
    
    println!("\n=== Summary (with proper IW_SHIFT) ===");
    println!("Total tests: {}", total_tests);
    println!("Perfect DC preservation: {}/{} ({:.1}%)", dc_perfect_tests, total_tests, dc_perfect_tests as f64 / total_tests as f64 * 100.0);
    println!("Perfect AC suppression: {}/{} ({:.1}%)", ac_perfect_tests, total_tests, ac_perfect_tests as f64 / total_tests as f64 * 100.0);
    println!("Perfect overall: {}/{} ({:.1}%)", perfect_tests, total_tests, perfect_tests as f64 / total_tests as f64 * 100.0);
    
    if perfect_tests == total_tests {
        println!("🎉 ALL TESTS PERFECT - DC preservation is working correctly!");
    } else {
        println!("⚠️  Issues detected - but may be acceptable for the shifted range");
    }
}
