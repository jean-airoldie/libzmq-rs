# Unprotocols

Protocols are contracts that allow peers to communicate. `ZeroMQ` enables
programmer to create such protocols that fit their needs at a very low cost.
But contracts are hard problems, whichever you look at them.

You might have realized by now that there is no strict concept of request-reply
as a socket operation. Indeed the library does not enforce a client socket
to follow a `send` call by a `recv` call. This does't mean however that this
strict type of request-reply could not be achieved. To do so, a programmer could
easily create the following method:

```rust
fn request_reply(msg: Msg, client: Client) -> Result<Msg, Error> {
    client.send(msg)?;
    client.recv_msg()?
}
```

`ZeroMQ` does not enforce a particular messaging protocol, instead
it offers all the tools to build one.
