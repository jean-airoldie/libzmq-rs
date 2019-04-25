`libzmq-rs` contains serializable builders which can be used in config files.
Note that all the fields are optional.

```toml
[radio]
# The following options are common to all socket types

# A list of endpoints to connect to.
connect = ["tcp://localhost:3000"]
# A list of endpoints to bind to.
bind = ["tcp://*:3001"]
backlog = 100
connect_timeout = 30000
heartbeat_interval = 3000
heartbeat_timeout = 3000
heartbeat_ttl = 3000

# The following options are for socket that impl `RecvMsg`

recv_high_water_mark = 1000
recv_timeout: 1000

# The following options are for socket that impl `SendMsg`

send_high_water_mark = 1000
send_timeout: 1000

# The following options exclusive to the `Dish` socket

groups = ["group_a", "group_b"]

# The following options exclusive to the `Radio` socket

no_drop = true
```
