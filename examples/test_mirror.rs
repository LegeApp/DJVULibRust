use djvu_encoder::encode::iw44::transform::mirror;

fn main() {
    println!("=== Mirror Function Test ===");
    
    // Test the specific case that was problematic
    let len = 8;
    for idx in -10..=10 {
        let result = mirror(idx, len);
        println!("mirror({}, {}) = {}", idx, len, result);
    }
    
    println!("\n=== Testing len=4 ===");
    let len = 4;
    for idx in -10..=10 {
        let result = mirror(idx, len);
        println!("mirror({}, {}) = {}", idx, len, result);
    }
}
