# Frequently Asked Questions

## Why are so many socket types currently not supported?
Currently only a fraction of all available ØMQ sockets are supported.
This because I determined that most of the sockets were deprecated.

To do so, I implemented every socket type (except `REQ` and `REP`)
and tested their functionality and their performance. A socket type
had to either offer exceptional performance in some domain or offer
a unique feature.

Turns out that you can replace all those deprecated
socket types in terms of functionality using only `Client`,
`Server`, `Dish` and `Radio`.

To test performance I wrote a benchmark using criterion that tested every basic
socket pattern (PUSH-PULL, EXCLUSIVE PAIR, etc.). I used a sample size of
50m messages per messages size of 10, 50 and 100 bytes, per pattern.
I determined that the thread safe type had equal or better single thread
performance in almost ALL cases. The only exception was the EXCLUSIVE PAIR
pattern which is used exclusively for inter-thread communication, where
you should rather be using something like [`crossbeam_channel`], which offers
significantly better performance.

Lastly these deprecated socket types had a ton of footguns that were
kept to maintain backward compatibility.

If you still think your favorite socket type should be supported, feel
free to open and issue as these decisions are not final.

[`crossbeam_channel`]: https://docs.rs/crossbeam-channel/0.3.8/crossbeam_channel/
