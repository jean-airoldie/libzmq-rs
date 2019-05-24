# Basics

These are the basic methods required to use a socket.

## Connect

The socket `connect` method is use to connect to a peer socket bound
at an endpoint to communicate with a peer. Usually a client socket will connect
to a server socket, but it could be the other way around.

```rust
let addr: TcpAddr = "8.8.8.8:420".try_into()?;
client.connect(addr)?;
```

Calling `connect` on a socket is not guaranteed to connect to the peer right
away. Usually, the actual connect call will be delayed until it is needed
(e.g. when sending a message).

Connections in `ZeroMQ` are different from traditional connections is the
sense that they automatically handle failure. For instance, if a connection
fails because of a network error, it will be automatically reconnected if
possible.

Furthermore, to successfully connect to a peer, the handshake corresponding to
the mechanism used must succeed. This handshake is also done in the background
and might fail for various reasons.

## Bind

The socket `bind` method is used to bind a local endpoint to accept connections
from peers. Usually a server socket will bind a known endpoint so that other socket
can connect to it.

```rust
let addr: TcpAddr = "127.0.0.1:*".try_into()?;
server.bind(addr)?;
```

Contrairy to `connect`, `bind` will attempt to bind to the endpoint straight
away. If the bind call succeeds, the socket will start to accept connections
attempts to this endpoint.

## Send

This a fundamental operation of a socket used to transfert messages to another
socket. To be able to send messages, a socket must implement the `SendMsg` trait.

```rust
client.send(msg)?;
```

When `send` is called on a socket, it will attempt to queue the message
to its outgoing buffer. If the buffer is full, meaning it has reached the
high water mark, the operation will block. If the `send_timeout` is set
to `None`, the operation will block until the buffer can accomodate for
the message. Otherwise if a duration is specified, it attempt to queue
the message for that duration and if it fails, return `WouldBlock`.
the timeout.

There is also the `try_send` method which will return with `WouldBlock` immediately
if it cannot queue the message.

Queued messages are send by a background I/O thread to the peer socket.
For the messages to be actually sent two conditions must be met:
* The connection with the peer socket is up.
* The peer socket can receive messages (its incoming buffer is not full).

Conceptually, a full outgoing buffer can mean many things:
* The connection has crashed temporarily (network error etc.)
* The peer socket has crashed and is restarting.
* The peer socket receives message slower than we can send
    them (thus this is a back throttling mechanism)
* Etc.

Many of these scenarios are conceptually indistinguishable. Therefore
the user has to decide what to do depending on the context.

## Recv

You guessed it, `recv` is a socket operation used to receive messages from
another socket. To be able to receive messages, a socket must implement
the `RecvMsg` trait.

```rust
let msg = client.recv_msg()?;
```

Calling `recv` on a socket will attempt to extract a message from its
incoming buffer. If the incoming buffer is empty, the operation will
block until a mesage is received in the buffer. If the `recv_timeout`
is specified, it will try to extract a message from the buffer for the
given duration and return `WouldBlock` if it failed.

There is also the `try_recv` method which, similarly to `try_send`, will return
with `WouldBlock` immediately if it cannot queue the message.

The incoming buffer receives message from the background I/O thread from the
peer socket. For the messages to be actually received two conditions must be met:
* The connection with the peer socket is up.
* The incoming buffer is not full.

Conceptually, an empty incoming buffer can mean many things:
* The socket can receive messages faster than what the peer can send.
* The peer has no messages to send.
* The connection has a network error.
* The peer has crashed
* Etc.

Like before, many of these scenarios are conceptually indistinguishable.
We have to decide what to do depending on the context.

