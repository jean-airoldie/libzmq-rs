# Frequently Asked Questions

## Why are so many socket types not supported?
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
performance in ALL cases. The only pattern that came even close was the
EXCLUSIVE PAIR pattern type which had exactly the same performance has two
`CLient` sockets.

Lastly these deprecated socket types had a ton of footguns that were
kept to maintan backward compatibility.
