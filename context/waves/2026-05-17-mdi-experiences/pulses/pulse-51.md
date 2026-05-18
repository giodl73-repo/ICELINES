# Pulse 51: Serve iPhone PWA shell

## Goal

Make the browser dashboard usable as the first iPhone version of IceLines by
shipping an installable mobile-web shell before taking on native iOS packaging.

## Changes

- Added PWA metadata served from `/static/site.webmanifest` with `/dashboard`
  as the standalone start URL.
- Added iOS home-screen metadata to the shared base template, including theme
  color, standalone capability, app title, and safe-area viewport support.
- Tightened narrow-screen dashboard CSS with horizontal-scroll top navigation,
  safe-area-aware sticky command chrome, touch-action hints, and smaller phone
  card spacing.
- Added static/template tests for the web manifest and mobile install contract.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web static_assets::tests::l0_assets_compile_in_non_empty`
- `cargo test -p icelines-web static_assets::tests::l0_style_css_carries_fit_class_contract`
- `cargo test -p icelines-web --test l1_static`
- `cargo test -p icelines-web templates::tests::l0_home_template_includes_a11y_baseline`
- `cargo test -p icelines-web templates::tests::l0_home_template_links_static_assets`
- `git diff --check`
- `cargo build --release`

## Status

Done.
