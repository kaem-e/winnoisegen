Simple tray icon app in rust exclusively for windows.


- It just embedds a simple [qoa](https://qoaformat.org/) audio file source of rain and plays it back. 
- Left click on the tray icon to toggle playback, Right click exits the app. Tooltip on the tray icon will indicate the current playback state
- Volume control is handled by the system mixer. icba to do smth for it
- If you want to encode your own sound source you can check the `encode_qoa.rs` example in the `examples` directory
- `.qoa` of rain audio file *i* use is included in the source tree, under `assets/rain.qoa`.
    > if you want to build from source and use your own sound you should put the resulting `.qoa` there as well


---

## Building


The app uses the currently unstable `portable_simd` feature. for this reason you will have to be on rust's nightly toolchain

```sh
rustup toolchain install nightly
cargo +nightly build
```

from there just copy the resulting binary over from `./target/release/winnoisegen.exe` to wherever you want and use whatever system you desire to orchestrate/automate starting it up
