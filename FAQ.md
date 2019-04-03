# Frequently Asked Questions

## Why are so many socket types not supported?
Currently we support only a fraction of all available ØMQ sockets.
This is because we consider some socket types to be completely eclipsed in
terms of fonctionality by newer sockets. Thus we won't currently support these
sockets in an effort to simplify and prevent misuse of the API. Feel free
to file an issue if you disagree with any of our choices.

## Why is `REQ` and `REP` not supported?
We don't consider their design to be really good. The fact that a `recv` and `send`
operations must alternate makes error handling horrendous. Not a fan of the
arbitrary empty frame before every message they send.

## Why is `DEALER` not supported?
It is replaced by the `Client` type (same functionality better performance,
thread-safety) except that they support multipart messages. But the multipart
behavior can be emulated by the `Client` socket by including metadata in the
serialized data. It also has some major footguns, including queueing messages
to uncompleted connections by default.

## Why is `ROUTER` not supported
It is replaced by the `Server` type (same functionality, better performance,
thread-safety) socket except that they support multipart.
Same explanation as `DEALER`. It also has major footguns.
For instance, by default it drops messages when in mute state or when they
cannot route messages. It also [leads to busy loops when polled](https://github.com/zeromq/libzmq/issues/2623).

## Why is `PUB` and `SUB` not supported
They are replaced by the `Sink` and `Dish` types.
