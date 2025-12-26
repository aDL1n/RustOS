Here you can find the source code for my operating system, written in Rust, based on the article "Writing an OS in Rust," as my first project in this language (not the best approach, but it's more interesting). If you're interested in implementing this version or would like to contribute, please create an issue.

# How to run
To build an image of this operating system and test it, you will need:
1. Rust nightly 1.91.0 and higher
2. QEMU emulator (You can also make a flash drive with an image)

## Build and Run
1. Build image 
```bash 
cargo bootimage --config D:\RustProjects\RustOS\.cargo\config.toml
```

2. Run image on QEMU
```bash
qemu-system-x86_64.exe -drive format=raw,file=target/rustos-target-x86-64/debug/bootimage-rust_os.bin
```