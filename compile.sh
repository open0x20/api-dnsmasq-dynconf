#!/bin/bash

# Target RaspberryPi 2 or lower
cargo build --release --target arm-unknown-linux-gnueabihf

# Target RaspberryPi 3 or higher
cargo build --release --target armv7-unknown-linux-gnueabihf
