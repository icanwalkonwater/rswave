# RPi Audio Led Visualizer (Very WIP)

Toy project to remotely control LED strips (WS281x) mounted on a RPi from another device using TCP sockets.

The visuals are driven by an audio input of the controlling device.

## Building
### Local crate
> Run with `--help` to see every available option.

The crate `rpi_led_local` is an executable to compile for the target `arm-unknown-linux-gnueabihf` and is to be ran on the Raspberry.

It is capable of controlling the LED bars and receives its instructions from a TCP socket using different protocols (`LedMode`) to control various parameters.

Refer to [this repo](https://github.com/rpi-ws281x/rpi-ws281x-rust) for cross-compilation stuff.

### Remote crate
> Run with `--help` to see every available option.

The crate `rpi_led_local` is to compile for whatever target you want. It is also an executable that will try to connect to the other app and will flood it with data.

Most of the heavy computations are happening on this side.

## Example setup
### RPi side
Start a server that will allow reconnections (`--multiple`) and with maximum brightness (255).

Depending on your configuration you might need to run it as root (to access GPIO pins).
```bash
cargo build
sudo target/debug/rpi_led_local --multiple --brightness 255
```

### Remote side
Start a client in intensity only mode (the led will be red but the amount of led on will depend on the volume of the audio source).
I have an audio input device named "Wave" that replays what is coming out of my computer so I will supply it as an hint to choose this input source.

Change `192.168.1.49` by the IP address of your raspberry.

This one can run in userspace without problem.
```bash
cargo build
target/debug/rpi_led_remote --only-intensity --device-pattern Wave 192.168.1.49:20200
```

## FAQ

> Some LEDs are still on after the end of the program.

Yes, turn them off using the `--reset` option.
