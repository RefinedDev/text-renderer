# text-renderer

A TrueType font renderer. Parses glyph outline data out of the font file and reconstructs each glyph's contours as quadratic Bezier curves, rendered using Bevy.

Built as a learning project to understand how fonts actually encode glyph shapes, and how a rendering engine turns that raw outline data into something drawn on screen. Inspired by Sebastian Lague's video on [Text Rendering](https://www.youtube.com/watch?v=SO83KQuuZvg).

## Showcase

<img width="580" height="169" alt="glyph rendering example" src="https://github.com/user-attachments/assets/eed7935d-4648-4aaf-9ed4-6b2ea813e428" />
<img width="791" height="388" alt="debug view showing contour points" src="https://github.com/user-attachments/assets/48cc27c2-b568-4da7-a00e-86b93909c223" />

## What it does

- Parses `.ttf` files
- Reconstructs each glyph's outline from the raw contour point data, stitching points together into quadratic Bezier curve segments
- Renders the resulting glyph shapes using Bevy

## Controls

- **Click** — move around the viewport
- **Scroll wheel** — zoom in on a glyph
- **Caps Lock** — toggle debug mode, showing all contour points and how they connect to form each glyph

## Notes
- TrueType fonts don't store glyph outlines as simple line segments, they store a set of on-curve and off-curve points, and the actual curve shape has to be reconstructed from that point data according to the format's rules (including implied on-curve points between consecutive off-curve points). Parsing that directly out of the binary font format, and turning it into properly stitched quadratic Bezier segments, was the core challenge here. The debug mode (Caps Lock) exists specifically to visualize that reconstruction, seeing the raw contour points and how they get connected into curves.
  
- Rendering is handled through Bevy, which is a full game engine. A lighter weight renderer would be the better choice for this
