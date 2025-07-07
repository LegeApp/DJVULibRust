#!/usr/bin/env python3
"""
Color Checker Script - Analyzes PPM files to verify color accuracy
"""
import sys
import os
from collections import Counter

def read_ppm(filename):
    """Read a PPM file and return pixel data"""
    try:
        with open(filename, 'rb') as f:
            # Read header
            magic = f.readline().decode().strip()
            if magic != 'P6':
                print(f"Error: {filename} is not a binary PPM file (P6)")
                return None
            
            # Skip comments
            line = f.readline().decode().strip()
            while line.startswith('#'):
                line = f.readline().decode().strip()
            
            # Parse dimensions
            width, height = map(int, line.split())
            
            # Read max color value
            max_val = int(f.readline().decode().strip())
            
            # Read pixel data
            pixel_data = f.read()
            
            print(f"File: {filename}")
            print(f"  Dimensions: {width}x{height}")
            print(f"  Max value: {max_val}")
            print(f"  Expected bytes: {width * height * 3}")
            print(f"  Actual bytes: {len(pixel_data)}")
            
            return {
                'width': width,
                'height': height,
                'max_val': max_val,
                'pixels': pixel_data
            }
    except Exception as e:
        print(f"Error reading {filename}: {e}")
        return None

def analyze_colors(ppm_data, filename, expected_color=None):
    """Analyze colors in PPM data"""
    if not ppm_data:
        return
    
    pixels = ppm_data['pixels']
    width = ppm_data['width']
    height = ppm_data['height']
    
    # Count unique colors
    color_counts = Counter()
    
    # Sample some pixels to check
    sample_pixels = []
    total_pixels = width * height
    
    for i in range(0, len(pixels), 3):
        if i + 2 < len(pixels):
            r = pixels[i]
            g = pixels[i + 1] 
            b = pixels[i + 2]
            color = (r, g, b)
            color_counts[color] += 1
            
            # Save first 10 pixels for detailed analysis
            if len(sample_pixels) < 10:
                sample_pixels.append(color)
    
    print(f"\n=== Color Analysis for {filename} ===")
    if expected_color:
        print(f"Expected color: RGB{expected_color}")
    
    print(f"Total unique colors: {len(color_counts)}")
    print(f"First 10 pixels: {sample_pixels}")
    
    # Show most common colors
    print("Most common colors:")
    for color, count in color_counts.most_common(5):
        percentage = (count / total_pixels) * 100
        print(f"  RGB{color}: {count} pixels ({percentage:.1f}%)")
    
    # Check if expected color matches
    if expected_color and expected_color in color_counts:
        expected_count = color_counts[expected_color]
        expected_percentage = (expected_count / total_pixels) * 100
        print(f"\n✅ Expected color RGB{expected_color} found: {expected_count} pixels ({expected_percentage:.1f}%)")
    elif expected_color:
        print(f"\n❌ Expected color RGB{expected_color} NOT found!")
        # Find closest colors
        closest_colors = []
        for color in color_counts.keys():
            distance = sum(abs(a - b) for a, b in zip(expected_color, color))
            closest_colors.append((distance, color, color_counts[color]))
        
        closest_colors.sort()
        print("Closest colors found:")
        for distance, color, count in closest_colors[:3]:
            percentage = (count / total_pixels) * 100
            print(f"  RGB{color} (distance {distance}): {count} pixels ({percentage:.1f}%)")

def main():
    # Test our generated PPM files
    test_files = [
        ("solid_blue.ppm", (0, 0, 255)),
        ("solid_red.ppm", (255, 0, 0)), 
        ("solid_green.ppm", (0, 255, 0))
    ]
    
    for filename, expected_color in test_files:
        if os.path.exists(filename):
            ppm_data = read_ppm(filename)
            analyze_colors(ppm_data, filename, expected_color)
        else:
            print(f"File {filename} not found")
    
    print("\n" + "="*50)

if __name__ == "__main__":
    main()
