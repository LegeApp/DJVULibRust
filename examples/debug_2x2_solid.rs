use djvu_encoder::encode::iw44::transform::Encode;

fn main() {
    println!("=== 2x2 Solid Color Transform Debug ===");
    
    // Test 2x2 solid color
    let mut data = vec![-448i32; 4]; // 2x2 solid color
    println!("Before transform: {:?}", data);
    
    // Apply 1 level transform
    Encode::forward::<1>(&mut data, 2, 2, 1);
    println!("After transform: {:?}", data);
    
    // For solid color, we expect:
    // DC = average = -448
    // All AC = 0
    let dc = data[0];
    let ac_coeffs: Vec<_> = data.iter().skip(1).collect();
    println!("DC: {}, AC coefficients: {:?}", dc, ac_coeffs);
    
    if dc == -448 && ac_coeffs.iter().all(|&&x| x == 0) {
        println!("✅ Perfect solid color transform!");
    } else {
        println!("❌ Transform error: DC should be -448, AC should be all zeros");
    }
}
