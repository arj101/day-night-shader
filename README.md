# day-night-shader

### To build and install locally
The SMFL dev library should be installed. On Debian:
```bash
sudo apt-get install libsfml-dev git
```

Rust must also be [installled](https://www.rust-lang.org/tools/install)

To download and build the executable:
```bash
git clone https://github.com/giovanni214/day-night-shader.git
cd day-night-shader
cargo build --release --bin day-night-shader-native #this will take a while
```

Your executable will be located in `[git dowload folder]/target/release/day-night-shader-native`
If you wish to install it locally:
```bash
cp /target/release/day-night-shader-native /usr/local/bin
```
