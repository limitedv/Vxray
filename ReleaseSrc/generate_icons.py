import sys
from PIL import Image

try:
    img = Image.open(r"C:\Users\sadra\.gemini\antigravity\brain\018cb3e5-4d2c-4cde-a57f-e761dd47f849\vxray_logo_1785758286722.jpg")
    img = img.resize((512, 512), Image.Resampling.LANCZOS)
    img.save("app-icon.png")
    print("app-icon.png created successfully!")
except Exception as e:
    print(f"Error: {e}")
