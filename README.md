# SO\_PROIRITY Proxy for TCP and UDP Connections

The proxy program intercepts TCP/UDP connections and adds
SO\_PRIORITY.

## Installation

`cargo` is required to install this program. If you haven't had it
yet, download it from [rustup.rs](https://rustup.rs/).

To install this program,

```bash
cargo install --git https://github.com/jerry73204/so-priority-proxy.git
```

## Usage

Create a list of proxy chains and save them in `proxies.txt`. For
example, to forward a TCP connection from local 55555 port to remote
port 55555 on server 3.1.4.1 and set its SO\_PRIORITY to 1, add this
entry to your list.

```
tcp | 1 | 127.0.0.1:55555 -> 3.1.4.1:55555
```

UDP connection is also supported. You can add one more UDP proxy to
your `proxies.txt` for example.

```
tcp | 1 | 127.0.0.1:55555 -> 3.1.4.1:55555
udp | 2 | 127.0.0.1:44444 -> 3.1.4.1:44444
```

To start the proxy program,

```bash
so-priority-proxy proxies.txt
```

## License

This software is distributed under MIT license. Please see the [LICENSE.txt](LICENSE.txt) file.
