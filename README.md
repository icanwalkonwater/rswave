# RSwave

Toy project to drive a LED strip using a realtime analysis of some audio combined with Spotify data.

## Architecture
* `rswave_server` contains the code that will control the LEDs, it is made to run on a raspberry and will listen for UDP datagrams to gather data and build patterns from it.
* `rswave_remote` contains the code that will perform the audio analysis and data fetching and send it via UDP datagrams to the RPi.
* `rswave_common` contains the code shared among the 2 other packages, mainly the structs serialisation/deserialization.

## Example Usage

### Server
For a WS2811 based led strip using GPIO18.
```bash
rswave_server -l ws2811
```

For a generic led strip controlled using GPIO23, GPIO24 and GPIO25 and port 1234.
```bash
rswave_server -l gpio -p 1234
```

### Remote
Obtain a spotify id and secret [here](https://developer.spotify.com/dashboard/).

Run remote only without communicating to the server and using the default audio source.
```bash
rswave_remote --spotify-id XXXXXXX --spotify-secret XXXXXXX
```

Run remote without an interface using the "Headphones" audio source and talking to a server on the same network.
```bash
rswave_remote --spotify-id XXXXXXX --spotify-secret XXXXXXX -d Headphones -a 192.168.0.20:20200 --no-tui
```

## Hack

### I want to support my own LED strip
Each type of led strip is controlled by a [`LedController`](./rswave_server/src/led_controllers.rs), you can implement your own by looking at the existing two.

You also need to add it to the [`LedStripType`](./rswave_server/src/lib.rs) enum and handle in the [main](./rswave_server/src/main.rs) and in the [sanity check](./rswave_server/build.rs).

### I want to add my own color patterns
Similarly to LedControllers, you can implement your own [`Runner`](./rswave_server/src/runners.rs), there are a lot of existing runners if you need examples.

You then need to actually use it in [`App#make_controller_thread`](./rswave_server/src/app.rs).

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