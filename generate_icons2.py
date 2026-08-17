import sys
from PIL import Image

try:
    img = Image.open(r"C:\Users\sadra\.gemini\antigravity\brain\018cb3e5-4d2c-4cde-a57f-e761dd47f849\vxray_logo_1785758286722.jpg")
    img = img.convert("RGBA")
    
    # Save as ICO (includes multiple sizes automatically if we want, or just one)
    # Windows typically wants 256, 128, 64, 48, 32, 16.
    # Let's save a good 256x256 ico.
    img_ico = img.resize((256, 256), Image.Resampling.LANCZOS)
    img_ico.save(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Rust\src-tauri\icons\icon.ico", format="ICO", sizes=[(256, 256)])
    
    # Save standard PNGs
    img.resize((32, 32), Image.Resampling.LANCZOS).save(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Rust\src-tauri\icons\32x32.png")
    img.resize((128, 128), Image.Resampling.LANCZOS).save(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Rust\src-tauri\icons\128x128.png")
    img.resize((256, 256), Image.Resampling.LANCZOS).save(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Rust\src-tauri\icons\128x128@2x.png")
    img.resize((512, 512), Image.Resampling.LANCZOS).save(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Rust\src-tauri\icons\icon.png")
    
    # ICNS isn't strictly necessary on Windows, but let's just make it a PNG rename to not break builds.
    img.resize((512, 512), Image.Resampling.LANCZOS).save(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Rust\src-tauri\icons\icon.icns", format="PNG")

    print("High-quality icons generated successfully!")
except Exception as e:
    print(f"Error: {e}")
