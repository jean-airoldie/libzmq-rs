# Secure Request Reply

The previous example did not offer neither authentication nor encryption.
For a public `TCP` connection, its a must. Let's fix that by adapting the
previous example.

This time we will the `CURVE` mechanism, which is a public-key crypto.
To enable the usage of the `CURVE` mechanism, the feature flag 'curve'
must be enabled.

However, this time we will use an external configuration file to get
rid of all the boilerplate. This will also allows our application
to run indepently of the socket configuration.

Based on some basic benchmarks, the `CURVE` mechanism
might reduce the throughtput of I/O heavy applications by half due
to the overhead of the `salsa20` encryption.

## Config File

In this case we used `yaml` configuration file, but any file format
supported by `Serde` will work (as long as it supports typed enums).
```yml
{{#include ../../../libzmq/examples/secure_req_rep.yml}}
```

## The code

Aside from the additionnal logic for reading the config file,
the code is now simpler than before.

{{#playpen ../../../libzmq/examples/secure_req_rep.rs}}
