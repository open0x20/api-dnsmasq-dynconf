#!/bin/bash

# Target RaspberryPi 2 or lower
cargo build --release --target arm-unknown-linux-gnueabihf
mkdir -p binaries/arm
cp target/arm-unknown-linux-gnueabihf/release/dnsmdcd binaries/arm/dnsmdcd


# Target RaspberryPi 3 or higher
cargo build --release --target armv7-unknown-linux-gnueabihf
mkdir -p binaries/armv7
cp target/armv7-unknown-linux-gnueabihf/release/dnsmdcd binaries/armv7/dnsmdcd
