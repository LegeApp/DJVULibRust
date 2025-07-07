#!/usr/bin/env python3
"""
Debug YCbCr conversion by analyzing what values should be produced
"""

def djvu_rgb_to_ycbcr(r, g, b):
    """
    Convert RGB to YCbCr using DjVu coefficients from the C++ encoder
    """
    # From IW44EncodeCodec.cpp:
    # Y  =  0.299 * R + 0.587 * G + 0.114 * B
    # Cb = -0.168 * R - 0.331 * G + 0.500 * B  
    # Cr =  0.500 * R - 0.418 * G - 0.081 * B
    
    # Convert to integer math matching C++:
    # rmul, gmul, bmul for Y channel (scaled by 65536):
    rmul_y = int(0.299 * 65536)  # 19595
    gmul_y = int(0.587 * 65536)  # 38469  
    bmul_y = int(0.114 * 65536)  # 7472
    
    # rmul, gmul, bmul for Cb channel:
    rmul_cb = int(-0.168 * 65536)  # -11010
    gmul_cb = int(-0.331 * 65536)  # -21693
    bmul_cb = int(0.500 * 65536)   # 32768
    
    # rmul, gmul, bmul for Cr channel:
    rmul_cr = int(0.500 * 65536)   # 32768
    gmul_cr = int(-0.418 * 65536)  # -27395
    bmul_cr = int(-0.081 * 65536)  # -5308
    
    # Calculate as in C++: ((rmul[r] + gmul[g] + bmul[b] + 32768) >> 16) - 128
    y_val = ((rmul_y * r + gmul_y * g + bmul_y * b + 32768) >> 16) - 128
    cb_val = ((rmul_cb * r + gmul_cb * g + bmul_cb * b + 32768) >> 16) - 128  
    cr_val = ((rmul_cr * r + gmul_cr * g + bmul_cr * b + 32768) >> 16) - 128
    
    return y_val, cb_val, cr_val

def test_colors():
    """Test the YCbCr conversion for our test colors"""
    test_cases = [
        ("Blue", 0, 0, 255),
        ("Red", 255, 0, 0), 
        ("Green", 0, 255, 0),
        ("White", 255, 255, 255),
        ("Black", 0, 0, 0),
    ]
    
    for name, r, g, b in test_cases:
        y, cb, cr = djvu_rgb_to_ycbcr(r, g, b)
        print(f"{name:>6} RGB({r:3}, {g:3}, {b:3}) -> YCbCr({y:4}, {cb:4}, {cr:4})")
        
        # Check if values are in valid i8 range
        if not (-128 <= y <= 127 and -128 <= cb <= 127 and -128 <= cr <= 127):
            print(f"  WARNING: Values out of i8 range!")

if __name__ == "__main__":
    test_colors()
