# FolioFS Brand Assets

Logo assets derived from the FolioFS Brand Guide v1.1.

## Files

```
icon/
  foliofs-icon.svg           Square icon, transparent background
  foliofs-icon-{1024,512,256,128}.png   PNG renders, transparent

wordmark/
  foliofs-wordmark-light.svg          "FolioFS" in ink — for paper / light backgrounds
  foliofs-wordmark-light-h{256,128,64}.png
  foliofs-wordmark-dark.svg           "FolioFS" in cream — for dark backgrounds
  foliofs-wordmark-dark-h{256,128,64}.png

lockup/
  foliofs-lockup-light.svg            Icon + wordmark, light-mode text
  foliofs-lockup-light-h{256,128,64}.png
  foliofs-lockup-dark.svg             Icon + wordmark, dark-mode text
  foliofs-lockup-dark-h{256,128,64}.png
```

## Choosing a variant

- **Light** (ink text) on backgrounds in the paper family — `#f1ece1`, `#e8e2d4`, `#d6cdb8`.
- **Dark** (cream text) on backgrounds in the dark-paper family — `#0f0c08`, `#1a1612`, `#2a241d`.
- The icon's vermillion shades stay the same across both modes — they have enough
  contrast on either background, so the icon doesn't need a separate variant.

## Colors

| Token              | Hex       | Used for                       |
| ------------------ | --------- | ------------------------------ |
| Vermillion         | `#d23822` | Icon primary triangle           |
| Vermillion (light) | `#e29281` | Icon accent triangle (50/50 mix with paper) |
| Ink                | `#15110c` | Wordmark on light backgrounds   |
| Ink (dark mode)    | `#f0e8d6` | Wordmark on dark backgrounds    |

## Typography

The brand guide specifies **Tiempos Text Regular** for the wordmark. Tiempos is a
commercial font (Klim Type Foundry), so for this open asset set the wordmark uses
**TeX Gyre Termes**, an open Times-family serif with comparable classical
proportions, bracketed serifs, and stroke contrast. All text is converted to
outlined paths in the SVGs, so no font file is needed at render time and the
wordmark looks identical anywhere it's displayed.

## SVG notes

- All SVGs use `viewBox` and have no fixed pixel `width`/`height`, so they scale
  cleanly to any size.
- The icon uses a `<clipPath>` so the diagonal crease meets the rounded
  bottom-left corner cleanly. Don't strip the `<defs>` block in optimization.
- Backgrounds are transparent — set your own background as needed.

## Lockup proportions

In the lockup, the icon's height equals **1.12 × cap height** of the wordmark and
is centered on the cap-height midline. The gap between icon and wordmark is
**0.30 × cap height**. These are baked into the SVG; if you need a different
ratio, regenerate from source.
