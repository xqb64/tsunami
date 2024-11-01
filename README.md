<h1 align="center">tsunami</h1>

This project represents my second attempt at writing a highly-performant TCP SYN reconnaissance tool. The objective was not to create the next `nmap`, but rather to explore the performance gains achievable by using Rust over Python. The process of putting it together was a real treat, and at this point I'm quite amazed by what Rust can do.

## How it works

The technique used, known as "stealth" (or "half-open") scanning, involves sending TCP packets with the `SYN` bit set. There are three pathways from here:

- If the target responds with `SYNACK`, it means that the port is open.
- If the target responds with `RSTACK`, the port is considered closed.
- If the target machine does not respond at all, `tsunami` will retry at most `--max-retries` times before reporting the port as filtered.

Upon receiving the response (in the first two cases), the kernel sends back another TCP packet with the RST bit set, effectively closing the connection in the middle of the handshake (hence "half-open").

## Let's talk numbers

In a lab environment on a machine with four cores and a direct 15m Category 6e link to the target router (`Asus RT-AC58U`, firmware `3.0.0.4.382_52134`), tsunami managed to inspect 64K ports in under 3 seconds.

```
$ time target/release/tsunami 192.168.1.1  -r 0-65535 --batch-size 32768 --flying-tasks 512 -n 10 -N 10
53: open
18017: open
34091: open
5473: open
515: open
9100: open
3838: open
3394: open
80: open
ports closed: 65527
ports filtered: 0
ports retried more than once: 7329

real	0m2,998s
user	0m1,058s
sys	0m0,893s
```

## Compiling

```
git clone https://github.com/xqb64/tsunami
cd tsunami
cargo build --release
```

This program requires `cap_net_raw`, so make sure you set that on the binary:

```
sudo setcap cap_net_raw+eip target/release/tsunami
```

## Usage

```
tsunami 0.1.0

USAGE:
    tsunami [OPTIONS] <target>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --batch-size <batch-size>               [default: 512]
    -f, --flying-tasks <flying-tasks>           [default: 512]
    -m, --max-retries <max-retries>             [default: 3]
    -N, --nap-after-batch <nap-after-batch>     [default: 10]
    -n, --nap-after-spawn <nap-after-spawn>     [default: 10]
    -p, --ports <ports>...
    -r, --ranges <ranges>...

ARGS:
    <target>
```

## Contributing

Contributions are very welcome, in particular, suggestions (and patches) as for how to make the whole system faster. Make sure you copy/paste the pre-commit hook into `.git/hooks`.

## References

- https://datatracker.ietf.org/doc/html/rfc791
- https://datatracker.ietf.org/doc/html/rfc9293

## See also

[wrath](https://github.com/xqb64/wrath) - My initial attempt, written in Python

## Licensing

Licensed under the [MIT License](https://opensource.org/licenses/MIT). For details, see [LICENSE](https://github.com/xqb64/tsunami/blob/master/LICENSE).
