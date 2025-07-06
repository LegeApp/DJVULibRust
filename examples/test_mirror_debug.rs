use djvu_encoder::encode::iw44::transform::mirror;

fn test_mirror_function() {
    println!("=== Testing mirror function ===");
    
    // Test mirror for length 2 (2x2 working area at level 1)
    println!("For length 2:");
    for i in -5..=5 {
        let result = mirror(i, 2);
        println!("  mirror({}, 2) = {}", i, result);
    }
    
    // Test mirror for length 4 (4x4 original area)
    println!("\nFor length 4:");
    for i in -5..=7 {
        let result = mirror(i, 4);
        println!("  mirror({}, 4) = {}", i, result);
    }
    
    // Specific test cases for the problematic scenario
    println!("\n=== Specific test for 2x2 vertical transform ===");
    println!("Working on 2x2 area, at odd position y=1:");
    println!("  above: mirror(0, 2) = {}", mirror(0, 2));  // y - s = 1 - 1 = 0
    println!("  below: mirror(2, 2) = {}", mirror(2, 2));  // y + s = 1 + 1 = 2
    
    println!("\nShould be: above=0, below=0 (both mirrored to row 0)");
    println!("So prediction should be (100 + 100) / 2 = 100");
    println!("And detail should be 100 - 100 = 0");
}

fn main() {
    test_mirror_function();
}
