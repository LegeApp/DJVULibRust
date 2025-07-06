use djvu_encoder::encode::iw44::transform::Encode;

fn main() {
    println!("=== 4x4 Solid Color Transform Debug ===");
    
    // Test 4x4 solid color with 1 level
    let mut data1 = vec![-448i32; 16]; // 4x4 solid color
    println!("Before transform (1 level): {:?}", data1);
    Encode::forward::<1>(&mut data1, 4, 4, 1);
    println!("After 1 level: {:?}", data1);
    let dc1 = data1[0];
    let non_zero_count1 = data1.iter().filter(|&&x| x != 0).count();
    println!("DC: {}, Non-zero coeffs: {}", dc1, non_zero_count1);
    
    println!();
    
    // Test 4x4 solid color with 2 levels
    let mut data2 = vec![-448i32; 16]; // 4x4 solid color
    println!("Before transform (2 levels): {:?}", data2);
    Encode::forward::<1>(&mut data2, 4, 4, 2);
    println!("After 2 levels: {:?}", data2);
    let dc2 = data2[0];
    let non_zero_count2 = data2.iter().filter(|&&x| x != 0).count();
    println!("DC: {}, Non-zero coeffs: {}", dc2, non_zero_count2);
    
    println!();
    
    // Test 8x8 solid color 
    let mut data3 = vec![-448i32; 64]; // 8x8 solid color
    println!("Before transform (8x8, 3 levels):");
    println!("First 8 values: {:?}", &data3[0..8]);
    let levels3 = ((8.min(8) as f32).log2() as usize).max(1);
    println!("Using {} levels", levels3);
    Encode::forward::<1>(&mut data3, 8, 8, levels3);
    println!("After {} levels:", levels3);
    println!("First 8 values: {:?}", &data3[0..8]);
    let dc3 = data3[0];
    let non_zero_count3 = data3.iter().filter(|&&x| x != 0).count();
    println!("DC: {}, Non-zero coeffs: {}", dc3, non_zero_count3);
    
    // Check if DC is correct
    if dc1 == -448 && dc2 == -448 && dc3 == -448 {
        println!("✅ All DC coefficients correct!");
    } else {
        println!("❌ DC coefficients wrong: {} {} {} (should be -448)", dc1, dc2, dc3);
    }
}
