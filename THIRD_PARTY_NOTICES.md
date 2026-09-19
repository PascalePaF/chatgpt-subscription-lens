# Third-party notices

订阅镜使用以下开源组件与资源。完整依赖版本可在 `native/Cargo.lock` 中审计。

## eframe / egui

- Project: <https://github.com/emilk/egui>
- License: MIT OR Apache-2.0
- Purpose: native Windows windowing, input, accessibility and GPU-rendered interface

## Noto Sans CJK SC

- Project: <https://github.com/notofonts/noto-cjk>
- Bundled file: `assets/fonts/NotoSansSC-Regular.otf`
- License copy: `assets/fonts/OFL-Noto-CJK.txt`
- License: SIL Open Font License 1.1

## Rust dependencies

Rust dependencies retain their respective upstream licenses. The dependency graph can be reproduced with:

```powershell
cargo tree --manifest-path native/Cargo.toml
```

## Generated interface assets

The four local 3D payment/store tiles under `assets/payment/` were generated specifically for this project. They contain no user data and are embedded into the executable. Visa, Mastercard, Apple App Store and Google Play names and marks belong to their respective owners; this project is not affiliated with or endorsed by those companies.
