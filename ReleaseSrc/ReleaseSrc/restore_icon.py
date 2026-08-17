import sys
from PIL import Image

try:
    img = Image.open(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Python\assets\icon.png")
    img = img.convert("RGBA")
    
    # Save as high-quality ICO
    img_ico = img.resize((256, 256), Image.Resampling.LANCZOS)
    img_ico.save(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Rust\src-tauri\icons\icon.ico", format="ICO", sizes=[(256, 256)])
    
    # Save standard PNGs
    img.resize((32, 32), Image.Resampling.LANCZOS).save(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Rust\src-tauri\icons\32x32.png")
    img.resize((128, 128), Image.Resampling.LANCZOS).save(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Rust\src-tauri\icons\128x128.png")
    img.resize((256, 256), Image.Resampling.LANCZOS).save(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Rust\src-tauri\icons\128x128@2x.png")
    img.resize((512, 512), Image.Resampling.LANCZOS).save(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Rust\src-tauri\icons\icon.png")
    img.resize((512, 512), Image.Resampling.LANCZOS).save(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Rust\src-tauri\icons\icon.icns", format="PNG")

    print("High-quality original icons generated successfully!")
except Exception as e:
    print(f"Error: {e}")
