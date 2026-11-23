//! Limits that stop the order form being used to send lots of email: so
//! many orders an hour from one visitor, and to one email address, and so
//! many a day for the whole shop. They're kept in memory, so they start
//! again whenever the service restarts.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const HOUR: Duration = Duration::from_secs(60 * 60);
const DAY: Duration = Duration::from_secs(24 * 60 * 60);

pub struct Limits {
    per_hour: u32,
    per_day: u32,
    sent: Mutex<Sent>,
}

/// When recent orders were sent: by visitor, by the buyer's address, and in all.
#[derive(Default)]
struct Sent {
    by_visitor: HashMap<String, VecDeque<Instant>>,
    by_address: HashMap<String, VecDeque<Instant>>,
    all: VecDeque<Instant>,
}

/// Which limit an order ran into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reached {
    /// The visitor, or the address, has sent `per_hour` orders in the last hour.
    Hour,
    /// The shop has taken `per_day` orders in the last day.
    Day,
}

/// A place taken by an order. If the order can't be sent after all, it's
/// given back.
#[derive(Debug)]
pub struct Place {
    visitor: String,
    address: String,
    at: Instant,
}

impl Limits {
    pub fn new(per_hour: u32, per_day: u32) -> Limits {
        Limits {
            per_hour,
            per_day,
            sent: Mutex::new(Sent::default()),
        }
    }

    /// Takes a place for an order from `visitor` (where it came from) with a
    /// copy to `address`, if the limits allow it.
    pub fn take(&self, visitor: &str, address: &str, now: Instant) -> Result<Place, Reached> {
        let mut sent = self
            .sent
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let address = address.to_lowercase();
        forget_before(&mut sent.all, now, DAY);
        if sent.all.len() >= self.per_day as usize {
            return Err(Reached::Day);
        }
        let per_hour = self.per_hour as usize;
        let busy =
            |times: &mut HashMap<String, VecDeque<Instant>>, key: &str| match times.get_mut(key) {
                Some(list) => {
                    forget_before(list, now, HOUR);
                    list.len() >= per_hour
                }
                None => false,
            };
        if busy(&mut sent.by_visitor, visitor) || busy(&mut sent.by_address, &address) {
            return Err(Reached::Hour);
        }
        sent.all.push_back(now);
        sent.by_visitor
            .entry(visitor.to_string())
            .or_default()
            .push_back(now);
        sent.by_address
            .entry(address.clone())
            .or_default()
            .push_back(now);
        // Visitors and addresses with nothing in the last hour are dropped,
        // so the lists don't grow for ever.
        sent.by_visitor
            .retain(|_, times| times.back().is_some_and(|t| now.duration_since(*t) < HOUR));
        sent.by_address
            .retain(|_, times| times.back().is_some_and(|t| now.duration_since(*t) < HOUR));
        Ok(Place {
            visitor: visitor.to_string(),
            address,
            at: now,
        })
    }

    /// Gives a place back, for an order that couldn't be sent.
    pub fn give_back(&self, place: Place) {
        let mut sent = self
            .sent
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let remove = |list: &mut VecDeque<Instant>| {
            if let Some(i) = list.iter().rposition(|t| *t == place.at) {
                list.remove(i);
            }
        };
        remove(&mut sent.all);
        if let Some(list) = sent.by_visitor.get_mut(&place.visitor) {
            remove(list);
        }
        if let Some(list) = sent.by_address.get_mut(&place.address) {
            remove(list);
        }
    }
}

fn forget_before(list: &mut VecDeque<Instant>, now: Instant, window: Duration) {
    while list
        .front()
        .is_some_and(|t| now.duration_since(*t) >= window)
    {
        list.pop_front();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_an_hour_from_one_visitor() {
        let limits = Limits::new(3, 20);
        let start = Instant::now();
        for n in 0..3 {
            let address = format!("buyer{n}@example.com");
            assert!(limits.take("203.0.113.7", &address, start).is_ok());
        }
        assert_eq!(
            limits
                .take("203.0.113.7", "other@example.com", start)
                .unwrap_err(),
            Reached::Hour
        );
        // Someone else is fine.
        assert!(
            limits
                .take("198.51.100.4", "other@example.com", start)
                .is_ok()
        );
        // An hour on, the first visitor can order again.
        assert!(
            limits
                .take("203.0.113.7", "next@example.com", start + HOUR)
                .is_ok()
        );
    }

    #[test]
    fn three_an_hour_to_one_address_whoever_sends_them() {
        let limits = Limits::new(3, 20);
        let now = Instant::now();
        for visitor in ["a", "b", "c"] {
            assert!(limits.take(visitor, "Mira@Example.com", now).is_ok());
        }
        assert_eq!(
            limits.take("d", "mira@example.com", now).unwrap_err(),
            Reached::Hour
        );
    }

    #[test]
    fn so_many_a_day_for_the_shop() {
        let limits = Limits::new(3, 4);
        let start = Instant::now();
        for n in 0..4 {
            let at = start + Duration::from_secs(n * 7_200);
            assert!(
                limits
                    .take(&n.to_string(), &format!("{n}@example.com"), at)
                    .is_ok()
            );
        }
        let later = start + Duration::from_secs(4 * 7_200);
        assert_eq!(
            limits.take("x", "x@example.com", later).unwrap_err(),
            Reached::Day
        );
        // A day after the first, there's room again.
        assert!(limits.take("x", "x@example.com", start + DAY).is_ok());
    }

    #[test]
    fn an_order_that_was_not_sent_gives_its_place_back() {
        let limits = Limits::new(1, 1);
        let now = Instant::now();
        let place = limits.take("a", "a@example.com", now).unwrap();
        assert!(limits.take("a", "a@example.com", now).is_err());
        limits.give_back(place);
        assert!(limits.take("a", "a@example.com", now).is_ok());
    }
}
