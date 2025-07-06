use djvu_encoder::encode::iw44::transform::{fwt_horizontal_inplace, fwt_vertical_inplace, mirror};

fn debug_horizontal_step(data: &mut [i32], w: usize, h: usize, level: usize) {
    println!("=== Horizontal Step Level {} ===", level);
    println!("Before horizontal:");
    for y in 0..h {
        for x in 0..w {
            print!("{:4} ", data[y * w + x]);
        }
        println!();
    }
    
    fwt_horizontal_inplace::<1>(data, w, h, level);
    
    println!("After horizontal:");
    for y in 0..h {
        for x in 0..w {
            print!("{:4} ", data[y * w + x]);
        }
        println!();
    }
}

fn debug_vertical_step(data: &mut [i32], w: usize, h: usize, level: usize) {
    println!("=== Vertical Step Level {} ===", level);
    println!("Before vertical:");
    for y in 0..h {
        for x in 0..w {
            print!("{:4} ", data[y * w + x]);
        }
        println!();
    }
    
    fwt_vertical_inplace::<1>(data, w, h, level);
    
    println!("After vertical:");
    for y in 0..h {
        for x in 0..w {
            print!("{:4} ", data[y * w + x]);
        }
        println!();
    }
}

fn main() {
    println!("=== Detailed Transform Debug for 8x8 Solid Color ===");
    
    let mut data = vec![-448i32; 64]; // 8x8 solid color
    let w = 8;
    let h = 8;
    
    println!("Initial 8x8 solid color image (all -448):");
    for y in 0..h {
        for x in 0..w {
            print!("{:4} ", data[y * w + x]);
        }
        println!();
    }
    println!();

    // Level 1: 8x8 -> 4x4
    debug_horizontal_step(&mut data, w, h, 1);
    println!();
    debug_vertical_step(&mut data, w, h, 1);
    println!();
    
    // Level 2: 4x4 -> 2x2 (work on the top-left 4x4)
    debug_horizontal_step(&mut data, w/2, h/2, 2);
    println!();
    debug_vertical_step(&mut data, w/2, h/2, 2);
    println!();
    
    // Level 3: 2x2 -> 1x1 (work on the top-left 2x2)
    debug_horizontal_step(&mut data, w/4, h/4, 3);
    println!();
    debug_vertical_step(&mut data, w/4, h/4, 3);
    println!();
    
    println!("Final result:");
    for y in 0..h {
        for x in 0..w {
            print!("{:4} ", data[y * w + x]);
        }
        println!();
    }
    
    println!("DC coefficient: {}", data[0]);
    let non_zero_count = data.iter().filter(|&&x| x != 0).count();
    println!("Non-zero coefficients: {}", non_zero_count);
}
