from PIL import Image, ImageEnhance
import sys

def enhance_image(path, out_path):
    try:
        img = Image.open(path)
        if img.mode != 'RGB':
            img = img.convert('RGB')
        
        # Enhance brightness
        enhancer = ImageEnhance.Brightness(img)
        img = enhancer.enhance(10.0) # 10x brightness
        
        # Enhance contrast
        enhancer = ImageEnhance.Contrast(img)
        img = enhancer.enhance(10.0) # 10x contrast
        
        img.save(out_path)
        print(f"Enhanced image saved to {out_path}")
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    if len(sys.argv) > 2:
        enhance_image(sys.argv[1], sys.argv[2])
    else:
        print("Usage: python enhance.py input.png output.png")
