# Reliable Request Reply

This is a basic example when using the `TCP` transport adapting the code
from the previous `Basic Request Reply` example.

Note that this example does not make any attempt at security.

Since `TCP` is connection oriented transport, we have to take in account that
the connection might fail at any time. We use heartbeating to detect failure
but also `send` and `recv` timeouts to prevent blocking forever.

In this example, the server is protected against failures since it will drop
messages if it is unable to route them before `send_timeout` expires (`WouldBlock`),
or it detects that the peer disconnected via the heartbeats (`HostUnreachable`).

The client in this case will simply fail if it unable to send a request before the
`send_timeout` or unable to receive a reply before the `recv_timeout` (`WouldBlock`).
The client might choose to retry later or connect to another server etc.

{{#playpen ../../../libzmq/examples/reliable_req_rep.rs}}
