# Basic Request Reply

This is as simple as it gets. We have a [Server] that does some request-reply
work in a dedicated thread. We have a [Client] that sends a "ping" and gets
a "pong" back. There is no attempt at security and no attempt at error handling.
For a `INPROC` server, that might enough.

{{#playpen ../../../libzmq/examples/basic_req_rep.rs}}

[Server]: https://docs.rs/libzmq/0.1/libzmq/struct.Server.html
[Client]: https://docs.rs/libzmq/0.1/libzmq/struct.Client.html
