# memflow-mirror

work in progress framebuffer mirror based on [memflow](https://github.com/memflow/memflow).

## Installation
Compile the guest-agent on Windows with:
```bash
cargo build --release --bin mirror-guest --all-features
```
Then run the mirror-guest.exe from the target/release/ directory.

In case you encounter a `No such file or directory` error from the build.rs script make sure to install the [dependencies of the winres crate](https://github.com/mxre/winres#toolkit).

Run the mirror tool with:
```bash
RUST_SETPTRACE=1 cargo run --release --bin mirror -- -vvv --connector kvm --process mirror-guest.exe
```

It is recommended to use the [memflow-kvm](https://github.com/memflow/memflow-kvm) connector as it currently has the best performance.

## Setup
### With memflow inventory
When running the mirror tool with the `default` features the memflow inventory will be used.
Since this project depends on memflow/next it is necessary to install [memflow-win32](https://github.com/memflow/memflow-win32) and a connector.
The simplest method currently is to use the [memflowup](https://github.com/memflow/memflowup) tool which has an interactive installation mode:
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.memflow.io | sh
```
Then follow the on-screen-instructions.

### Without memflow inventory
You can also specify the `memflow-static` feature when building the mirror tool.
This will statically link [memflow-win32](https://github.com/memflow/memflow-win32) as well as [memflow-qemu](https://github.com/memflow/memflow-qemu/tree/next) into the resulting binary. Just run the mirror tool with:
```bash
RUST_SETPTRACE=1 cargo run --release --bin mirror --features memflow-static -- -vvv --connector qemu --process mirror-guest.exe
```

### Development
For development purposes you can enable the `shader-reload` feature which uses the [notify](https://github.com/notify-rs/notify) crate to hot reload shaders. To run the tool with this feature enabled just do:
```bash
RUST_SETPTRACE=1 cargo run --release --bin mirror --features shader-reload -- -vvv --connector qemu --process mirror-guest.exe
```

## Demo

[![mirror demo](http://img.youtube.com/vi/H-1wxAeocGA/0.jpg)](http://www.youtube.com/watch?v=H-1wxAeocGA "mirror demo")

## Credits

[dxgcap](https://github.com/bryal/dxgcap-rs) by [bryal](https://github.com/bryal)

[obs-rs](https://github.com/not-matthias/obs-rs) by [not-matthias](https://github.com/not-matthias)

## License

Licensed under MIT License, see [LICENSE](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, shall be licensed as above, without any additional terms or conditions.
