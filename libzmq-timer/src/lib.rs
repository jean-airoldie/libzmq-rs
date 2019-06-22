//! Timer implementation.
//!
//! This module contains the types needed to run a timer.
//!
//! The [`Timer`] type runs the timer logic. It holds all the necessary state
//! to track all associated [`Delay`] instances and delivering notifications
//! once the deadlines are reached.
//!
//! The [`Handle`] type is a reference to a [`Timer`] instance. This type is
//! `Clone`, `Send`, and `Sync`. This type is used to create instances of
//! [`Delay`].
//!
//! The [`Now`] trait describes how to get an [`Instant`] representing the
//! current moment in time. [`SystemNow`] is the default implementation, where
//! [`Now::now`] is implemented by calling [`Instant::now`].
//!
//! [`Timer`] is generic over [`Now`]. This allows the source of time to be
//! customized. This ability is especially useful in tests and any environment
//! where determinism is necessary.
//!
//! Note, when using the Tokio runtime, the [`Timer`] does not need to be manually
//! setup as the runtime comes pre-configured with a [`Timer`] instance.
//!
//! [`Timer`]: struct.Timer.html
//! [`Handle`]: struct.Handle.html
//! [`Delay`]: ../struct.Delay.html
//! [`Now`]: ../clock/trait.Now.html
//! [`Now::now`]: ../clock/trait.Now.html#method.now
//! [`SystemNow`]: struct.SystemNow.html
//! [`Instant`]: https://doc.rust-lang.org/std/time/struct.Instant.html
//! [`Instant::now`]: https://doc.rust-lang.org/std/time/struct.Instant.html#method.now

mod level;
mod wheel;

use level::*;
use wheel::*;

use libzmq::{prelude::*, *};
use quanta::{Clock, Mock};

use std::{time::Duration, sync::Arc};

pub struct Interval {
    unit: Duration,
    repeat: Quantity,
}

pub struct Request {
    id: u64,
}

pub struct Reply {
}

/// Timer implementation that drives [`Delay`], [`Interval`], and [`Timeout`].
///
/// A `Timer` instance tracks the state necessary for managing time and
/// notifying the [`Delay`] instances once their deadlines are reached.
///
/// It is expected that a single `Timer` instance manages many individual
/// [`Delay`] instances. The `Timer` implementation is thread-safe and, as such,
/// is able to handle callers from across threads.
///
/// Callers do not use `Timer` directly to create [`Delay`] instances.  Instead,
/// [`Handle`][Handle.struct] is used. A handle for the timer instance is obtained by calling
/// [`handle`]. [`Handle`][Handle.struct] is the type that implements `Clone` and is `Send +
/// Sync`.
///
/// After creating the `Timer` instance, the caller must repeatedly call
/// [`turn`]. The timer will perform no work unless [`turn`] is called
/// repeatedly.
///
/// The `Timer` has a resolution of one millisecond. Any unit of time that falls
/// between milliseconds are rounded up to the next millisecond.
///
/// When the `Timer` instance is dropped, any outstanding [`Delay`] instance that
/// has not elapsed will be notified with an error. At this point, calling
/// `poll` on the [`Delay`] instance will result in `Err` being returned.
///
/// # Implementation
///
/// `Timer` is based on the [paper by Varghese and Lauck][paper].
///
/// A hashed timing wheel is a vector of slots, where each slot handles a time
/// slice. As time progresses, the timer walks over the slot for the current
/// instant, and processes each entry for that slot. When the timer reaches the
/// end of the wheel, it starts again at the beginning.
///
/// The `Timer` implementation maintains six wheels arranged in a set of levels.
/// As the levels go up, the slots of the associated wheel represent larger
/// intervals of time. At each level, the wheel has 64 slots. Each slot covers a
/// range of time equal to the wheel at the lower level. At level zero, each
/// slot represents one millisecond of time.
///
/// The wheels are:
///
/// * Level 0: 64 x 1 millisecond slots.
/// * Level 1: 64 x 64 millisecond slots.
/// * Level 2: 64 x ~4 second slots.
/// * Level 3: 64 x ~4 minute slots.
/// * Level 4: 64 x ~4 hour slots.
/// * Level 5: 64 x ~12 day slots.
///
/// When the timer processes entries at level zero, it will notify all the
/// [`Delay`] instances as their deadlines have been reached. For all higher
/// levels, all entries will be redistributed across the wheel at the next level
/// down. Eventually, as time progresses, entries will [`Delay`] instances will
/// either be canceled (dropped) or their associated entries will reach level
/// zero and be notified.
///
/// [`Delay`]: ../struct.Delay.html
/// [`Interval`]: ../struct.Interval.html
/// [`Timeout`]: ../struct.Timeout.html
/// [paper]: http://www.cs.columbia.edu/~nahum/w6998/papers/ton97-timing-wheels.pdf
/// [`handle`]: #method.handle
/// [`turn`]: #method.turn
/// [Handle.struct]: struct.Handle.html
#[derive(Debug)]
pub struct Timer {
    /// The frontend server socket.
    server: Server,

    /// The instant at which the timer started running.
    start: u64,

    /// The last published timer `elapsed` value.
    elapsed: u64,

    /// Timer wheel
    wheel: wheel::Wheel,

    /// Source of "now" instances
    clock: Clock,
}

impl Timer {
    /// Create a new `Timer` instance that uses `park` to block the current
    /// thread and `clock` to get the current `Instant`.
    ///
    /// Specifying the source of time is useful when testing.
    pub fn new(server: Server) -> Self {
        Self {
            server,
            start: 0,
            elapsed: 0,
            wheel: wheel::Wheel::new(),
            clock: Clock::new(),
        }
    }

    pub fn with_mock_clock(server: Server) -> (Self, Arc<Mock>) {
        let (clock, mock) = Clock::mock();
        (Self {
            server,
            start: 0,
            elapsed: 0,
            wheel: wheel::Wheel::new(),
            clock: Clock::new(),
        }, mock)
    }

    pub fn run(&mut self) -> Result<(), Error> {
        loop {
            if now > next_tick {
                self.process()
            } else {
                match self.server.try_recv_msg() {
                    Ok(msg) => {
                        let id = msg.routing_id().unwrap();
                    }
                    Err(err) => {
                        match err.kind() {
                            ErrorKind::WouldBlock => {
                                thread::sleep(next_tick - now);
                            }
                            _ => Err(err),
                        }
                    }
                }
            }
        }
    }

    /// Converts an `Expiration` to an `Instant`.
    fn expiration_instant(&self, when: u64) -> Instant {
        self.inner.start + Duration::from_millis(when)
    }

    /// Run timer related logic
    fn process(&mut self) {
        let now = crate::ms(self.now.now() - self.inner.start, crate::Round::Down);
        let mut poll = wheel::Poll::new(now);

        while let Some(entry) = self.wheel.poll(&mut poll, &mut ()) {
            let when = entry.when_internal().expect("invalid internal entry state");
            unimplemented!();
        }

        // Update the elapsed cache
        self.inner.elapsed = self.wheel.elapsed();
    }

    /// Process the entry queue
    ///
    /// This handles adding and canceling timeouts.
    fn process_queue(&mut self) {
        for entry in self.inner.process.take() {
            match (entry.when_internal(), entry.load_state()) {
                (None, None) => {
                    // Nothing to do
                }
                (Some(_), None) => {
                    // Remove the entry
                    self.clear_entry(&entry);
                }
                (None, Some(when)) => {
                    // Queue the entry
                    self.add_entry(entry, when);
                }
                (Some(_), Some(next)) => {
                    self.clear_entry(&entry);
                    self.add_entry(entry, next);
                }
            }
        }
    }

    fn clear_entry(&mut self, entry: &Arc<Entry>) {
        self.wheel.remove(entry, &mut ());
        entry.set_when_internal(None);
    }

    /// Fire the entry if it needs to, otherwise queue it to be processed later.
    ///
    /// Returns `None` if the entry was fired.
    fn add_entry(&mut self, entry: Arc<Entry>, when: u64) {
        use crate::wheel::InsertError;

        entry.set_when_internal(Some(when));

        match self.wheel.insert(when, entry, &mut ()) {
            Ok(_) => {}
            Err((entry, InsertError::Elapsed)) => {
                // The entry's deadline has elapsed, so fire it and update the
                // internal state accordingly.
                entry.set_when_internal(None);
                entry.fire(when);
            }
            Err((entry, InsertError::Invalid)) => {
                // The entry's deadline is invalid, so error it and update the
                // internal state accordingly.
                entry.set_when_internal(None);
                entry.error();
            }
        }
    }
}
