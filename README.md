# tsunami

A highly performant reconnaissance tool built for inspecting sizable port ranges quickly while minimizing the detection risk at the same time.

![tsunami](tsunami.png)

The technique used, known as "stealth" (or "half-open") scanning, involves sending TCP packets with the SYN bit set. If the target responds with SYNACK, it means that the port is open -- otherwise, if it responds with RSTACK, it means that the port is closed. Upon receiving the response, the kernel sends back another TCP packet with the RST bit set, effectively closing the connection in the middle of the handshake (hence "half-open"). 

## Performance

In a lab environment on a machine with four cores and a direct 15m Category 6e link to the target router, tsunami managed to inspect ~64K ports in under 3 seconds.

TODO: implement proper throttling and rate limiting, ensuring that all ports that were given are actually inspected.