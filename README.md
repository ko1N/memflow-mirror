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

## Setup
### With memflow inventory
When running the mirror tool with the `default` features the memflow inventory will be used.
Since this project depends on memflow/next it is necessary to install an appropiate connector like qemu_procfs:
```bash
git clone https://github.com/memflow/memflow-qemu-procfs
cd memflow-qemu-procfs
git checkout next
./install.sh --system
```

The OS Plugin for win32 has to be installed as well:
```bash
git clone https://github.com/memflow/memflow
cd memflow
git checkout next
cargo build --release --all-features --workspace
sudo cp target/release/libmemflow_win32.so /usr/lib/memflow/
```

### Without memflow inventory
You can also specify the `memflow-static` feature when building the mirror tool.
This will statically link memflow-win32 as well as memflow-qemu-procfs into the resulting binary. Just run the mirror tool with:
```bash
RUST_SETPTRACE=1 cargo run --release --bin mirror --all-features -- -vvv --connector qemu_procfs --process mirror_guest.exe
```

## Demo

[![mirror demo](http://img.youtube.com/vi/H-1wxAeocGA/0.jpg)](http://www.youtube.com/watch?v=H-1wxAeocGA "mirror demo")

