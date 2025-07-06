use djvu_encoder::encode::iw44::transform::{fwt_horizontal_inplace, fwt_vertical_inplace};

fn main() {
    println!("=== Simple Transform Debug ===");
    
    // Test full 4x4 transform with different level counts
    for levels in 1..=2 {
        let mut buf = vec![-3840_i32; 16]; // 4x4 constant
        println!("\n4x4 with {} levels:", levels);
        println!("Before: {:?}", buf);
        
        fwt_horizontal_inplace::<4>(&mut buf, 4, 4, levels);
        println!("After horizontal: {:?}", buf);
        
        fwt_vertical_inplace::<4>(&mut buf, 4, 4, levels);
        println!("After vertical: {:?}", buf);
        
        // Count non-zero AC coefficients (everything except [0,0])
        let dc = buf[0];
        let non_zero_ac = buf[1..].iter().filter(|&&x| x != 0).count();
        
        println!("DC: {}, Non-zero AC coefficients: {}", dc, non_zero_ac);
    }
    
    // Also test what the max_levels calculation gives us
    let max_levels_4x4 = ((4.min(4) as f64).log2().floor() as usize).max(1);
    println!("\nMax levels for 4x4: {}", max_levels_4x4);
}
