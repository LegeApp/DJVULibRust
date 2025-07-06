// Debug the update step in detail

use djvu_encoder::encode::iw44::transform::mirror;

fn debug_update_step_8x8() {
    println!("=== Debugging update step for 8x8 solid color ===");
    
    // Simulate the state after predict step for level 0 (s=1, dbl=2)
    // For solid color, all detail coefficients should be 0
    let mut col_vec = vec![100i32; 8];  // Original solid color column
    let mut detail = vec![0i32; 8];     // All details are 0 for solid color
    
    let s = 1;   // scale for level 0
    let dbl = 2; // stride
    let h = 8;
    
    // Manually set detail coefficients for odd positions (should be 0 for solid color)
    detail[1] = 0;  // position 1
    detail[3] = 0;  // position 3
    detail[5] = 0;  // position 5
    detail[7] = 0;  // position 7
    
    println!("Before update step:");
    println!("  col_vec: {:?}", col_vec);
    println!("  detail:  {:?}", detail);
    
    // Apply update step manually for each even position
    let mut y = 0;
    while y < h {
        println!("\nEven position y={}", y);
        
        // Calculate mirrored indices
        let d_idx_m2 = mirror(y as isize - 2 * s as isize, h);  // d[n-2]
        let d_idx_m1 = mirror(y as isize - s as isize, h);      // d[n-1]
        let d_idx_0  = mirror(y as isize, h);                   // d[n]
        let d_idx_p1 = mirror(y as isize + s as isize, h);      // d[n+1]
        
        println!("  Mirrored indices: m2={}, m1={}, 0={}, p1={}", d_idx_m2, d_idx_m1, d_idx_0, d_idx_p1);
        
        // Check conditions
        let cond_m2 = d_idx_m2 % dbl == s && d_idx_m2 < h;
        let cond_m1 = d_idx_m1 % dbl == s && d_idx_m1 < h;
        let cond_0  = d_idx_0 % dbl == s && d_idx_0 < h;
        let cond_p1 = d_idx_p1 % dbl == s && d_idx_p1 < h;
        
        println!("  Conditions: m2={}, m1={}, 0={}, p1={}", cond_m2, cond_m1, cond_0, cond_p1);
        
        // Get detail values
        let d_m2 = if cond_m2 { detail[d_idx_m2] } else { 0 };
        let d_m1 = if cond_m1 { detail[d_idx_m1] } else { 0 };
        let d_0  = if cond_0 { detail[d_idx_0] } else { 0 };
        let d_p1 = if cond_p1 { detail[d_idx_p1] } else { 0 };
        
        println!("  Detail values: m2={}, m1={}, 0={}, p1={}", d_m2, d_m1, d_0, d_p1);
        
        let upd = (d_m2 + 9 * d_m1 + 9 * d_0 - d_p1 + 16) >> 5;
        
        println!("  Update value: {}", upd);
        println!("  Original col_vec[{}]: {}", y, col_vec[y]);
        
        col_vec[y] += upd;
        
        println!("  New col_vec[{}]: {}", y, col_vec[y]);
        
        y += dbl;
    }
    
    println!("\nAfter update step:");
    println!("  col_vec: {:?}", col_vec);
    
    // Copy detail back
    for y in (s..h).step_by(dbl) {
        col_vec[y] = detail[y];
    }
    
    println!("After copying detail back:");
    println!("  col_vec: {:?}", col_vec);
}

fn main() {
    debug_update_step_8x8();
}
