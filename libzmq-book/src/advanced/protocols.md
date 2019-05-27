# Custom Protocols

For two peers to be able to communicate, they must share a contract. In the
world of communication, these are called protocols. `ZeroMQ` enables
programmer to create protocols that suit their needs by removing most of the
boilerplate.

You might have realized by now that there is no strict concept of request-reply
as a socket operation. Indeed the library does not enforce a client socket
to follow a `send` call by a `recv` call. This does't mean however that this
strict type of request-reply could not be achieved. To do so, a programmer could
easily write the following code:

```rust
// Client side
fn request_reply(&mut self, msg: Msg) -> Result<Msg, Error> {
    self.client.send(msg)?;
    self.client.recv_msg()?
}

// Server side
fn run(&mut self) -> Result<(), Error> {
    loop {
        let request = self.server.recv_msg()?;
        let reply = self.on_request(request)?;
        self.server.send(reply)
    }
}
```

This creates an implicit contract between the client and the server.
We will disregard the error handling and timeouts for simplicity.
* The client must send one request at a time and wait for one reply.
* The server must wait for a request and send one reply.

Since contract must be ensured at the application level, it must be properly
documentated for developpers to be able to respect it.

`ZeroMQ` does not enforce a particular messaging protocol, instead
it offers all the tools to build one. But in the end, good contracts are hard
to pull of, whichever way you look at them.
