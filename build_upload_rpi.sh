#!/usr/bin/env bash

# Pretty specific script, please don't run it blindly

cargo build -p rswave_server --release --target armv7-unknown-linux-gnueabihf \
  && scp target/armv7-unknown-linux-gnueabihf/release/rswave_server rpi:rswave \
  && ssh rpi
