# Sift demo data (English)

**Made-up** files for the English screenshots and video of the development build: an online shop for a client, database and networking notes, the flat's bills and a trip. They belong to nobody.

- With `SIFT_DEV_DEMO=en`, the development build copies this folder to `C:\Users\Public\Documents\Sift (demo)`, indexes it and creates a few example Spaces. No personal path shows on screen. See `demo_folder` in `apps/desktop/src-tauri/src/dev.rs`.
- The Spanish version is in `demo/` (`SIFT_DEV_DEMO=1`).
- The two PDFs are generated with `python scripts/demo-pdf.py demo-en en`.
