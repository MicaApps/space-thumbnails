from PIL import Image
import os

path = r"D:\Users\Shomn\OneDrive - MSFT\Source\Repos\space-thumbnails\crates\windows\assets\loading.png"
# Blue circle on transparent background
img = Image.new('RGBA', (256, 256), (0, 0, 0, 0))
pixels = img.load()
center = 128
radius = 100
for x in range(256):
    for y in range(256):
        if (x - center)**2 + (y - center)**2 < radius**2:
            pixels[x, y] = (0, 120, 215, 255) # Windows Blue

img.save(path)
print(f"Created {path}")
