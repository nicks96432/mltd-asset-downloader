# MLTD asset manifest tool

This crate contains MLTD asset manifest downloading functions and other tools related to
manifest and versions.

## Example

Download latest Android manifest:

```rust
use std::path::PathBuf;

use mltd_asset_manifest::{Manifest, Platform};

let manifest = Manifest::from_version(&Platform::Android, None).unwrap();
manifest.save(&PathBuf::from(&manifest.asset_version.filename)).unwrap();
```
