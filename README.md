# api-dnsmasq-dynconf
A service to manage dnsmasq entries/records via HTTP API.

## Endpoints

| Method | Path    | Description                                                              |
|--------|---------|--------------------------------------------------------------------------|
| GET    | /list   | Returns a JSON formatted list of entries from /etc/dnsmasq.d/custom.conf |
| PUT    | /add    | Adds an entry to /etc/dnsmasq.d/custom.conf                              |
| POST   | /delete | Removes an entry from /etc/dnsmasq.d/custom.conf                         |

Every request (except /list, which is public) requires the following payload:
```
{
    "name": "",
    "ip": "",
    "secret": ""
}
```
It is also required to set the header `Content-Type: application/json` for all requests with a json payload.
Otherwise a `400 Bad Request` will be returned.

### Examples
```
GET /list


HTTP/1.1 200 OK
{
    "addresses": [
        {
            "name": "test1.example.com",
            "ip": "127.0.0.1"
        },
        {
            "name": "test2.example.com",
            "ip": "127.0.0.2"
        },
    ]
}
```
```
PUT /add
Content-Type: application/json

{
    "name": "test3.example.com",
    "ip": "127.0.0.3",
    "secret": "ABCDEF"
}

HTTP/1.1 200 OK
```
```
POST /delete
Content-Type: application/json

{
    "name": "test3.example.com",
    "ip": "127.0.0.3",
    "secret": "ABCDEF"
}

HTTP/1.1 200 OK
```

Some negative examples aswell...
```
PUT /add
Content-Type: application/json

{
    "name": "test3.example.com",
    "ip": "127.0.0.3",
    "secret": "WRONG_SECRET"
}

HTTP/1.1 401 Unauthorized
```
```
PUT /add
Content-Type: application/json

{
    "name": "test3.example.com"
}

HTTP/1.1 400 Bad Request
```

## Setup
The following files are required prior to startup. They will be created empty if missing:
```
/etc/dnsmdcd.token          # contains your secret token
/etc/dnsmasq.d/custom.conf  # create as empty file
```
Owner of the files should be root as the program is expecting root privileges. The service will listen on
127.0.0.1:47078. Use a reverse proxy for HTTPS.

### Binaries
Precompiled binaries can be found [here](https://binaries.open0x20.de/api-dnsmasq-dynconf).

SHA256 hashes:
```
sha256sum -b FILE
latest/arm/dnsmdcd   - b0ce7f5e488da382379a36fdc5871832ed24bc8c5a08016e641cc262d58809d7
latest/armv7/dnsmdcd - 263fcfbfd0fdcc2a6d562d5dfeb9673fdbd2114eb179e165978a7ac0b37fa890
```

### Building
Either compile on the target itself or install a cross compiler.

Read more on cross compiling rust [here](https://chacin.dev/blog/cross-compiling-rust-for-the-raspberry-pi/).

Install the target architecture:
```
# RaspberryPi 2 or lower
rustup target add arm-unknown-linux-gnueabihf

# RaspberryPi 3 or higher
rustup target add armv7-unknown-linux-gnueabihf
```

#### Compile on RaspberryPi
After the architecture has been installed with `rustup`, simply run the following:
```
# For RaspberryPi 2 or lower
cargo build --release --target arm-unknown-linux-gnueabihf

# For RaspberryPi 3 or higher
cargo build --release --target armv7-unknown-linux-gnueabihf

```

#### Cross-Compiling
Install the cross-compiler (debian):
```
# Not tested if existent
apt install arm-linux-gnueabihf-gcc
apt install armv7-linux-gnueabihf-gcc
```

Install the cross-linker (debian):
```
TODO
```

### Installation
Copy the `dnsmdcd.service` file into `/etc/systemd/system/` and the binary `dnsmdcd` into `/usr/sbin/`. Then you can simply run the following
commands to start/stop the service:
```
systemctl start dnsmdcd.service
systemctl stop dnsmdcd.service
```
