import sys
try:
    from PIL import Image
    img = Image.open(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Python\assets\icon.png")
    img.save(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Rust\src-tauri\icons\icon.ico")
    img.save(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Rust\src-tauri\icons\32x32.png")
    img.save(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Rust\src-tauri\icons\128x128.png")
    img.save(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Rust\src-tauri\icons\128x128@2x.png")
    img.save(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Rust\src-tauri\icons\icon.icns")
    # For good measure, let's also put icon.png in the same directory as well.
    img.save(r"c:\Users\sadra\OneDrive\Vxray\Vxray-Rust\src-tauri\icons\icon.png")
    print("Icons generated successfully!")
except Exception as e:
    print(f"Error: {e}")
