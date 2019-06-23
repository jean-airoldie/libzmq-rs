# Socket

The concept of a `socket` in `ZeroMQ` is completely novel. A `ZeroMQ` socket
differs from a traditional `TCP` socket in the following ways (but not limited to):

* A socket sends and receives atomic messages; messages are guaranteed to
    either be transmitted in their entirety, or not transmitted at all.
* A socket send and receive messages asynchronously.
* A socket can transmit messages over many supported transports, including `TCP`.
* Incoming and outgoing messages can be queued and transmitted asynchronously
    by a background I/O thread.
* A socket can be connected to zero or more peers at any time.
* A socket can be bound to zero or more endpoints at any time. Each bound
    endpoint can listen to zero or more peers.
* Peer reconnection and disconnection is handled in the background.
* Support for many authentication and encryption strategies via [Mechanism].

[Mechanism]: https://docs.rs/libzmq/0.1/libzmq/auth/enum.Mechanism.html
