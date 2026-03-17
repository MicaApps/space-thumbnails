from PIL import Image
import sys

def check_image(path):
    try:
        img = Image.open(path)
        if img.mode != 'RGB':
            img = img.convert('RGB')
        
        # Get pixels
        pixels = list(img.getdata())
        
        # Check if all pixels are black or very dark
        is_all_black = all(sum(p) < 30 for p in pixels)
        
        # Get average color
        avg_r = sum(p[0] for p in pixels) / len(pixels)
        avg_g = sum(p[1] for p in pixels) / len(pixels)
        avg_b = sum(p[2] for p in pixels) / len(pixels)
        
        print(f"Path: {path}")
        print(f"Is all black: {is_all_black}")
        print(f"Avg color: ({avg_r:.1f}, {avg_g:.1f}, {avg_b:.1f})")
        print(f"Dimensions: {img.size}")
    except Exception as e:
        print(f"Error checking {path}: {e}")

if __name__ == "__main__":
    if len(sys.argv) > 1:
        check_image(sys.argv[1])
    else:
        print("Please provide an image path.")
