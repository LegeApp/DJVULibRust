use djvu_encoder::encode::iw44::transform::Encode;

fn test_level_by_level_debug() {
    println!("=== Level-by-level Debug for 4x4 solid color 100 ===");
    
    let mut data = vec![100; 16]; // 4x4 image
    println!("Initial: {:?}", data);
    
    // Do one level at a time manually to see what happens
    let mut cur_w = 4;
    let mut cur_h = 4;
    
    for level in 0..2 {
        println!("\n--- Before level {} (area: {}x{}) ---", level, cur_w, cur_h);
        println!("Data: {:?}", data);
        
        // Manually call the single-level functions to debug
        djvu_encoder::encode::iw44::transform::fwt_vertical_single_level::<1>(&mut data, 4, cur_w, cur_h);
        println!("After vertical: {:?}", data);
        
        djvu_encoder::encode::iw44::transform::fwt_horizontal_single_level::<1>(&mut data, 4, cur_w, cur_h);
        println!("After horizontal: {:?}", data);
        
        cur_w = (cur_w + 1) / 2;
        cur_h = (cur_h + 1) / 2;
        
        println!("Next area will be: {}x{}", cur_w, cur_h);
    }
    
    println!("\n--- Final result ---");
    println!("DC: {}", data[0]);
    let mut ac_count = 0;
    for i in 1..data.len() {
        if data[i] != 0 {
            ac_count += 1;
            println!("AC[{}] = {}", i, data[i]);
        }
    }
    println!("Non-zero ACs: {}", ac_count);
}

fn main() {
    test_level_by_level_debug();
}
