# api-dnsmasq-dynconf

## Installation
Requires the following two files:
```
/etc/dnsmasq-dynconf.token       # contains your secret token
/etc/dnsmasq.d/custom.conf       # create as empty file
```
Owner of the files should be root as the program is expecting root privileges.

## Endpoints

Every endpoints expects the following query parameters: name, ip and secret.

| Method | Path    | Description                                                              |
|--------|---------|--------------------------------------------------------------------------|
| GET    | /list   | Returns a JSON formatted list of entries from /etc/dnsmasq.d/custom.conf |
| PUT    | /add    | Adds an entry to /etc/dnsmasq.d/custom.conf                              |
| POST   | /delete | Removes an entry from /etc/dnsmasq.d/custom.conf                         |

### Examples
```
GET /list?name=&ip=&secret=ABCDEF

HTTP/1.1 200 OK
{
    "addresses": [
        {
            "address": "test1.myhost.de",
            "ip": "127.0.0.1"
        },
        {
            "address": "test2.myhost.de",
            "ip": "127.0.0.2"
        },
    ]
}
```
```
PUT /add?name=test3.myhost.de&ip=127.0.0.3&secret=ABCDEF

HTTP/1.1 200 OK
```
```
POST /delete?name=test3.myhost.de&ip=127.0.0.3&secret=ABCDEF

HTTP/1.1 200 OK
```
Some negative examples aswell...
```
GET /list?name=&ip=&secret=WRONG

HTTP/1.1 401 Unauthorized
```
```
PUT /add?missing=parameters&equals=true

HTTP/1.1 400 Bad Request
```