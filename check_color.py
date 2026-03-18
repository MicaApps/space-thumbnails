from PIL import Image
import sys

def check_color(path):
    img = Image.open(path).convert('RGB')
    pixels = img.load()
    width, height = img.size
    
    color_pixel_count = 0
    total_pixels = width * height
    
    for y in range(height):
        for x in range(width):
            r, g, b = pixels[x, y]
            if max(r, g, b) - min(r, g, b) > 20: 
                color_pixel_count += 1
    
    print(f"Colorful pixels: {color_pixel_count} / {total_pixels} ({color_pixel_count/total_pixels*100:.2f}%)")
    
    if color_pixel_count > 0:
        # Find the most "colorful" pixel (max saturation)
        max_sat = 0
        best_pixel = (0, 0, 0)
        for y in range(height):
            for x in range(width):
                r, g, b = pixels[x, y]
                sat = max(r, g, b) - min(r, g, b)
                if sat > max_sat:
                    max_sat = sat
                    best_pixel = (r, g, b)
        print(f"Most colorful pixel: RGB{best_pixel} (saturation: {max_sat})")
        return True
    
    if not has_color:
        print("Image is grayscale (or black and white).")
    return False

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python check_color.py <image_path>")
        sys.exit(1)
    check_color(sys.argv[1])
