Simple tray icon app in rust exclusively for windows.


- It just embedds a simple [qoa](https://qoaformat.org/) audio file source of rain and plays it back. 
- Left click on the tray icon to toggle playback, Right click exits the app. Tooltip on the tray icon will indicate the current playback state
- Volume control is handled by the system mixer. icba to do smth for it
- `.qoa` of rain audio file *i* use is included in the source tree. 
- If you want to encode your own sound source you can check the `encode_qoa.rs` example in the `examples` directory


---

## Building


The app uses the currently unstable `portable_simd` feature. for this reason you will have to be on rust's nightly toolchain

```sh
rustup toolchain install nightly
cargo +nightly build
```

from there just copy the resulting binary over from `./target/release/winnoisegen.exe` to wherever you want and use whatever system you desire to orchestrate/automate starting it up
