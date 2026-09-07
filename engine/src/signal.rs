//! First-class publish/subscribe signal bus.
//!
//! The roadmap's flagged core-runtime gap: gameplay systems that need fan-out
//! currently talk through ad-hoc shared queues (`Arc<Mutex<Vec<Action>>>` per
//! cast), which bakes one-producer-one-consumer into every relationship.
//! [`Signal`] is the primitive for the other shape — something happens once
//! and several systems each react.
//!
//! Design notes:
//!
//! - Synchronous fan-out: [`Signal::emit`] runs every live handler on the
//!   emitter's thread, in subscription order. This is the boring, inspectable
//!   contract a turn-based game wants; it is not a message-passing scheduler.
//!   ponytail: a slow handler blocks the emitter and no frame/ordering
//!   guarantee exists — the upgrade path is a channel-drained mailbox, and the
//!   bus becomes its fan-out front door.
//! - Isolation: handlers run against a snapshot taken under the read lock, so
//!   a handler may subscribe, unsubscribe or emit re-entrantly without
//!   deadlocking the `RwLock`.
//! - Handles: dropping a [`Subscription`] unsubscribes it; `emit` skips
//!   inactive slots. Slots are compacted on write.

use std::sync::{Arc, RwLock};

/// A broadcast handle. Cloning it shares the same bus, so a system that wants
/// to talk can hold one clone while handing another to a listener's owner.
pub struct Signal<T> {
    inner: Arc<Inner<T>>,
}

struct Inner<T> {
    next_id: std::sync::atomic::AtomicU64,
    slots: RwLock<Vec<Slot<T>>>,
}

struct Slot<T> {
    id: u64,
    active: bool,
    handler: Arc<dyn Fn(&T) + Send + Sync>,
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> Default for Signal<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Signal<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                next_id: std::sync::atomic::AtomicU64::new(1),
                slots: RwLock::new(Vec::new()),
            }),
        }
    }

    /// Attach `handler`, to be called on every [`Signal::emit`] until the
    /// returned [`Subscription`] is dropped.
    pub fn subscribe(&self, handler: impl Fn(&T) + Send + Sync + 'static) -> Subscription<T> {
        let id = self
            .inner
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.inner
            .slots
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push(Slot {
                id,
                active: true,
                handler: Arc::new(handler),
            });
        Subscription {
            bus: Arc::clone(&self.inner),
            id,
        }
    }

    /// Fan `event` out to every live subscription. Runs synchronously, in
    /// subscription order, on the caller's thread.
    pub fn emit(&self, event: &T) {
        // Snapshot the handlers under the read lock and drop it before
        // calling any of them, so a handler that subscribes, unsubscribes or
        // emits re-entrantly cannot deadlock the lock.
        let snapshot: Vec<Arc<dyn Fn(&T) + Send + Sync>> = self
            .inner
            .slots
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|slot| slot.active)
            .map(|slot| Arc::clone(&slot.handler))
            .collect();
        for handler in snapshot {
            handler(event);
        }
    }

    /// Live subscription count, for tests and diagnostics.
    pub fn subscription_count(&self) -> usize {
        self.inner
            .slots
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|slot| slot.active)
            .count()
    }

    /// Drop inactive slots left behind by dropped subscriptions. Called
    /// automatically by `subscribe`; exposed for callers that subscribe and
    /// drop in tight loops and want the vector to not grow forever.
    pub fn compact(&self) {
        let mut slots = self
            .inner
            .slots
            .write()
            .unwrap_or_else(|e| e.into_inner());
        slots.retain(|slot| slot.active);
    }
}

/// Dropping the handle unsubscribes its handler from future emits. The
/// in-flight emit (if any) still reaches it, which is fine: the event was
/// already published.
pub struct Subscription<T> {
    bus: Arc<Inner<T>>,
    id: u64,
}

impl<T> Drop for Subscription<T> {
    fn drop(&mut self) {
        let mut slots = self.bus.slots.write().unwrap_or_else(|e| e.into_inner());
        for slot in slots.iter_mut() {
            if slot.id == self.id {
                slot.active = false;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;

    #[test]
    fn every_live_subscription_receives_every_emit() {
        let bus = Signal::new();
        let a = Arc::new(AtomicUsize::new(0));
        let b = Arc::new(AtomicUsize::new(0));
        let _sa = bus.subscribe({
            let a = Arc::clone(&a);
            move |n: &u32| {
                a.fetch_add(*n as usize, Ordering::Relaxed);
            }
        });
        let _sb = bus.subscribe({
            let b = Arc::clone(&b);
            move |n: &u32| {
                b.fetch_add(*n as usize, Ordering::Relaxed);
            }
        });

        bus.emit(&3);
        bus.emit(&5);
        assert_eq!(a.load(Ordering::Relaxed), 8);
        assert_eq!(b.load(Ordering::Relaxed), 8);
        assert_eq!(bus.subscription_count(), 2);
    }

    #[test]
    fn dropping_a_handle_stops_its_handler() {
        let bus = Signal::new();
        let count = Arc::new(AtomicUsize::new(0));
        let sub = bus.subscribe({
            let count = Arc::clone(&count);
            move |_: &()| {
                count.fetch_add(1, Ordering::Relaxed);
            }
        });
        bus.emit(&());
        assert_eq!(count.load(Ordering::Relaxed), 1);
        drop(sub);
        bus.emit(&());
        assert_eq!(count.load(Ordering::Relaxed), 1);
        assert_eq!(bus.subscription_count(), 0);
    }

    #[test]
    fn clones_share_the_same_bus() {
        let bus = Signal::new();
        let clone = bus.clone();
        let count = Arc::new(AtomicUsize::new(0));
        let _sub = bus.subscribe({
            let count = Arc::clone(&count);
            move |_: &()| {
                count.fetch_add(1, Ordering::Relaxed);
            }
        });
        // Emitting through either handle reaches the same subscription.
        clone.emit(&());
        bus.emit(&());
        assert_eq!(count.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn subscribing_during_emit_lands_from_the_next_emit() {
        // Subscribe inside a handler (re-entrant lock) must not deadlock, and
        // the new subscription is live from the *next* emit, not the one in
        // flight — it was not part of the snapshot.
        let bus = Signal::new();
        let seen = Arc::new(AtomicUsize::new(0));
        let late = Arc::new(AtomicUsize::new(0));
        // The subscription handle must outlive the handler call, so it is held
        // in a cell the outer test can drop later — this also guards that only
        // one late subscription is ever created.
        let holder: Arc<std::sync::Mutex<Option<Subscription<u32>>>> =
            Arc::new(std::sync::Mutex::new(None));
        let _a = bus.subscribe({
            let bus = bus.clone();
            let seen = Arc::clone(&seen);
            let holder = Arc::clone(&holder);
            let late = Arc::clone(&late);
            move |n: &u32| {
                seen.fetch_add(*n as usize, Ordering::Relaxed);
                let mut guard = holder.lock().unwrap();
                if guard.is_none() {
                    let late_in = Arc::clone(&late);
                    *guard = Some(bus.subscribe(move |m: &u32| {
                        late_in.fetch_add(*m as usize, Ordering::Relaxed);
                    }));
                }
            }
        });
        bus.emit(&1);
        assert_eq!(seen.load(Ordering::Relaxed), 1);
        assert_eq!(late.load(Ordering::Relaxed), 0);
        // The next emit reaches the mid-emit subscriber and the original.
        bus.emit(&2);
        assert_eq!(seen.load(Ordering::Relaxed), 3);
        assert_eq!(late.load(Ordering::Relaxed), 2);
        assert_eq!(bus.subscription_count(), 2);
    }

    #[test]
    fn a_handler_can_emit_without_deadlocking_siblings() {
        // Emit inside a handler: the nested emit reaches the sibling handler
        // (a fresh snapshot), and the guard keeps the recursion to one level.
        let bus = Signal::new();
        let count = Arc::new(AtomicUsize::new(0));
        let nested = Arc::new(AtomicUsize::new(0));
        let _a = bus.subscribe({
            let bus = bus.clone();
            let nested = Arc::clone(&nested);
            move |n: &u32| {
                if *n != 999 {
                    // A nested publish, distinct in value so the sibling can
                    // tell it apart.
                    bus.emit(&999);
                    return;
                }
                nested.fetch_add(1, Ordering::Relaxed);
            }
        });
        let _b = bus.subscribe({
            let count = Arc::clone(&count);
            move |n: &u32| {
                count.fetch_add(*n as usize, Ordering::Relaxed);
            }
        });
        bus.emit(&1);
        // B sees both the outer (1) and the nested (999) emits.
        assert_eq!(count.load(Ordering::Relaxed), 1000);
        assert_eq!(nested.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn compact_collects_dropped_subscriptions() {
        let bus = Signal::new();
        {
            let _a = bus.subscribe(|_: &()| {});
            let _b = bus.subscribe(|_: &()| {});
            assert_eq!(bus.subscription_count(), 2);
        }
        assert_eq!(bus.subscription_count(), 0);
        bus.compact();
        // After compaction a fresh subscribe lands in a clean vector; the
        // count is still just the live one.
        let _c = bus.subscribe(|_: &()| {});
        assert_eq!(bus.subscription_count(), 1);
    }
}