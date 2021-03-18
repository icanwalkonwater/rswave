# RSwave

Toy project to drive a LED strip using a realtime analysis of some audio combined with Spotify data.

## Architecture
* `rswave_server` contains the code that will control the LEDs, it is made to run on a raspberry and will listen for UDP datagrams to gather data and build patterns from it.
* `rswave_remote` contains the code that will perform the audio analysis and data fetching and send it via UDP datagrams to the RPi.
* `rswave_common` contains the code shared among the 2 other packages, mainly the structs serialisation/deserialization.

## Cross compilation
Building `rswave_server` on the RPi can take a long time, fortunately cross compilation is an option.

You'll need the packages `clang-dev` and `gcc-arm-linux-gnueabihf`. You also need to configure cargo by putting this in `~/.cargo/config`:
```toml
[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"
```

Next install the target through rustup:
```bash
rustup target add armv7-unknown-linux-gnueabihf
```

Now you can compile the package and copy paste it to your RPi.
```bash
cargo build -p rswave_server --release --target armv7-unknown-linux-gnueabihf
```