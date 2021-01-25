#!/bin/bash

# Target RaspberryPi 2 or lower
cargo build --release --target arm-unknown-linux-gnueabihf
mkdir -p binaries/arm
cp target/arm-unknown-linux-gnueabihf/release/dnsmasq-dynconf binaries/arm/dnsmasq-dynconf


# Target RaspberryPi 3 or higher
cargo build --release --target armv7-unknown-linux-gnueabihf
mkdir -p binaries/armv7
cp target/armv7-unknown-linux-gnueabihf/release/dnsmasq-dynconf binaries/armv7/dnsmasq-dynconf
