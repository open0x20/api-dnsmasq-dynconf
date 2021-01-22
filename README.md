# api-dnsmasq-dynconf

## Installation
Requires the following two files:
```
/etc/dnsmasq-dynconf.token       # contains your secret token
/etc/dnsmasq.d/custom.conf       # create as empty file
```
Owner of the files should be root as the program is expecting root privileges.

## Endpoints

| Method | Path    | Description                                                              |
|--------|---------|--------------------------------------------------------------------------|
| GET    | /list   | Returns a JSON formatted list of entries from /etc/dnsmasq.d/custom.conf |
| PUT    | /add    | Adds an entry to /etc/dnsmasq.d/custom.conf                              |
| POST   | /delete | Removes an entry from /etc/dnsmasq.d/custom.conf                         |

Every request (except to /list, which is public) requires the following payload:
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
PUT /add
Content-Type: application/json

{
    "name": "test3.myhost.de",
    "ip": "127.0.0.3",
    "secret": "ABCDEF"
}

HTTP/1.1 200 OK
```
```
POST /delete
Content-Type: application/json

{
    "name": "test3.myhost.de",
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
    "name": "test3.myhost.de",
    "ip": "127.0.0.3",
    "secret": "WRONG_SECRET"
}

HTTP/1.1 401 Unauthorized
```
```
PUT /add
Content-Type: application/json

{
    "name": "test3.myhost.de"
}

HTTP/1.1 400 Bad Request
```