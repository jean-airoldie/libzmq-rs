# Glossary
Some high level definitions.

## Endpoint
A endpoint is a rendez-vous address for a specified transport. The syntax
of the address depends on the nature of the transport. For instance
a `TcpAddr` is an endpoint over the `TCP` transport.

## Transport
A protocol to transfert data. Could be a network protocol, such as `TCP`,
could be a inter-thread protocol, such as `INPROC`, etc.

## Connection
Connections in `ZeroMQ` are different from traditional connections is the
sense that they automatically handle failure. For instance, if a connection
fails because of a network error, it will be automatically reconnected if
possible. Thus sockets should not worry about the state of a given connection.

## Message
An atomic arbitrary set of bytes owned by the `ZeroMQ` engine. `ZeroMQ` does
not know how to interpret these bytes, only the user does. Messages are
the units that are transferred between sockets.

## Socket
A `ZeroMQ` construct used to send and receive messages using connections
accros endpoints. The specific behavior of the socket depends on its type.

## High Water Mark
The message limit in the incoming or outgoing buffer. If the incoming
buffer has reached this limit, the socket will stop receiving messages
in the background. If the outgoing buffer has reached this limit, attempting
to queue a message will block the calling thread. Conceptually, this is a
socket's back throttling mechanism.

## Context
A `ZeroMQ` context is a session that keeps track of all the sockets,
the messages, the async threads and the internal queries.

## Mechanism
A specific protocol used by sockets to authenticate and encrypt traffic.

## Mute State
A socket that is in mute state is unable to queue and receive messages.
This is likely because it has no peers. The condition for the mute state to
occur depends on the socket type.
