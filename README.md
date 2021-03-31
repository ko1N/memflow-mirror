# memflow-mirror

work in progress framebuffer mirror based on [memflow](https://github.com/memflow/memflow).

## Installation
Compile the guest-agent on Windows with:
```bash
cargo build --release --bin mirror_guest --all-features
```

Run the mirror tool with:
```bash
RUST_SETPTRACE=1 cargo run --release --bin mirror -- -vvv --connector qemu_procfs --process mirror_guest.exe
```
