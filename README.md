# day-night-shader

### To Build and Install Locally
Rust must be [installled.](https://www.rust-lang.org/tools/install)
The SMFL dev library should also be installed. On Debian:
```bash
sudo apt-get install libsfml-dev git
```

<br>
To download and build the executable:
```bash
git clone https://github.com/giovanni214/day-night-shader.git
cd day-night-shader
cargo build --release --bin day-night-shader-native #this will take a while
```

<br>
Your executable will be located in `[git dowload folder]/target/release/day-night-shader-native`
If you wish to install it locally:
```bash
cp /target/release/day-night-shader-native /usr/local/bin
```
