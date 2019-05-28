# Patterns

These are the most basic socket patterns in `libzmq`.

## Client-Server

The `Client-Server` pattern is a advanced asynchronous request-reply pattern.

The [Server] receives messages with a unique [RoutingId] associated with a
[Client]. This [RoutingId] can be used by the [Server] to route replies to the
[Client].
```
       <───> client
server <───> client
       <───> client

```

The [Client] socket receives upstream messages in a fair-queued fashion
```
server ─┐
server ────> client
server ─┘
```

## Radio-Dish

The `Radio-Dish` pattern is an asynchronous publish-subscribe pattern.

The [Radio] socket send messages in a fan-out fashion to all [Dish]
that [joined] the message's [Group].
```
      ────> dish
radio ────> dish
      ────> dish
```

The [Dish] socket receive messages from [Group] it has [joined] in a
fair-queued fashion.
```
radio ─┐
radio ────> dish
radio ─┘
```

## Scatter-Gather

The `Scatter-Gather` pattern is an asynchronous pipeline pattern.

The [Scatter] socket send messages downstream in a round-robin fashion
```
         ┌──> gather
scatter ────> gather
         └──> gather
```

The [Gather] socket receives upstream messages in a fair-queued fashion
```
scatter ─┐
scatter ───> gather
scatter ─┘
```

[Server]: https://docs.rs/libzmq/0.1/libzmq/struct.Server.html
[RoutingId]: https://docs.rs/libzmq/0.1/libzmq/struct.RoutingId.html
[Client]: https://docs.rs/libzmq/0.1/libzmq/struct.Client.html
[Radio]: https://docs.rs/libzmq/0.1/libzmq/struct.Radio.html
[Dish]: https://docs.rs/libzmq/0.1/libzmq/struct.Dish.html
[Group]: https://docs.rs/libzmq/0.1/libzmq/struct.Group.html
[Scatter]: https://docs.rs/libzmq/0.1/libzmq/struct.Scatter.html
[Gather]: https://docs.rs/libzmq/0.1/libzmq/struct.Gather.html
[joined]: https://docs.rs/libzmq/0.1/libzmq/struct.Dish.html#method.join
