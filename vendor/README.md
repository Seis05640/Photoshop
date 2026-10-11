# Vendored crates

Permissively licensed crates we carry small patches for. Each directory keeps the upstream
licence files unchanged. Every patch is marked `PhotoCraft patch:` in the source, is covered by a
PhotoCraft test, and should be proposed upstream. Remove the vendored copy once upstream releases
the fix.

| Crate | Version | Licence | Patches | Test |
|---|---|---|---|---|
| winit | 0.30.13 | Apache-2.0 | (1) on X11, without Xft DPI, use the X server's configured screen DPI before the output's physical density. Xorg can deliberately run a dense panel at 96 DPI; using its EDID dimensions instead makes only winit applications appear zoomed. Keep configured HiDPI screens and the explicit `randr` override working. | `apps/photocraft/tests/winit_x11_dpi.rs`; behavior tests in `util::randr::tests` |
