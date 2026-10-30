//! Layout transactions, as sway's (`desktop/transaction.c`): when an arrange gives
//! windows new sizes, every tile keeps being drawn where it was until those windows have
//! drawn their new size, or `TIMEOUT` goes by. Then the whole layout moves in one frame,
//! so neighbours never show a gap or an overlap while a slow app catches up.
//!
//! Arranges during one join it: it keeps where the tiles were drawn when it began and
//! waits for the new windows too. A resize sliding into place (`animate_resize`) starts once
//! it ends, from there.

use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use smithay::reexports::wayland_server::{Resource, backend::ObjectId};

use crate::{Dune, layout::{Rect, TileId}};

/// How long a layout waits for its windows, like sway's default.
pub const TIMEOUT: Duration = Duration::from_millis(200);

pub struct Transaction {
    /// Where every tile was drawn when it began: they stay there meanwhile.
    from: HashMap<TileId, Rect>,
    /// The windows that have not drawn their new size yet.
    waiting: HashSet<ObjectId>,
    started: Instant,
    /// A resize slide was asked for meanwhile: it starts at the end.
    animate: bool,
}

impl Transaction {
    /// Where `id` is drawn while it lasts; `None`: a tile it does not hold (a new one).
    pub fn held(&self, id: TileId) -> Option<Rect> {
        self.from.get(&id).copied()
    }

    pub fn ask_animation(&mut self) {
        self.animate = true;
    }

}

/// Whether a transaction that began at `started` and still waits for `waiting` windows is
/// over at `now`.
fn due(waiting: usize, started: Instant, now: Instant) -> bool {
    waiting == 0 || now.duration_since(started) >= TIMEOUT
}

/// The windows an arrange waits for: those of `sent` (tiles laid out at `to`, with the
/// window each was sent a size) whose size differs from where they were drawn (`from`)
/// and that still have a size to draw (`pending`).
pub fn waiting_for<K: Clone + Eq + std::hash::Hash>(
    from: &HashMap<TileId, Rect>,
    to: &[(TileId, Rect)],
    sent: &HashMap<TileId, K>,
    pending: impl Fn(&K) -> bool,
) -> HashSet<K> {
    to.iter()
        .filter(|(id, rect)| from.get(id).is_some_and(|was| (was.w, was.h) != (rect.w, rect.h)))
        .filter_map(|(id, _)| sent.get(id))
        .filter(|key| pending(key))
        .cloned()
        .collect()
}

impl Dune {
    /// Before an arrange lays tiles out: where they are drawn now, for a transaction to
    /// hold them (a running one keeps its own).
    pub fn drawn_now(&self) -> HashMap<TileId, Rect> {
        self.geometries.iter().map(|(id, r)| (*id, self.drawn_rect(*id, *r))).collect()
    }

    /// After an arrange sent the sizes: hold the layout at `before` while windows draw
    /// theirs, joining the transaction running if there is one. Nothing to wait for: the
    /// layout shows as it is.
    pub fn begin_transaction(&mut self, before: HashMap<TileId, Rect>) {
        let sent: HashMap<TileId, ObjectId> = self
            .geometries
            .iter()
            .filter_map(|(id, _)| Some((*id, self.tiles.get(id)?.top()?.toplevel()?.wl_surface().id())))
            .collect();
        let from = self.transaction.as_ref().map(|t| t.from.clone()).unwrap_or(before);
        let waits = &self.configure_waits;
        let waiting = waiting_for(&from, &self.geometries, &sent, |key| waits.contains_key(key));
        match &mut self.transaction {
            Some(transaction) => transaction.waiting.extend(waiting),
            None if !waiting.is_empty() => {
                tracing::trace!(windows = waiting.len(), "layout waits for its windows");
                self.transaction = Some(Transaction { from, waiting, started: Instant::now(), animate: false });
            }
            None => {}
        }
    }

    /// The window `key` drew the size it was sent.
    pub fn transaction_drawn(&mut self, key: &ObjectId) {
        if let Some(transaction) = &mut self.transaction {
            transaction.waiting.remove(key);
        }
    }

    /// Before a frame: the transaction ends once its windows are drawn or it timed out,
    /// and the layout shows (sliding there, when a resize asked for it). `true`: it ended.
    pub fn step_transaction(&mut self) -> bool {
        let waits = &self.configure_waits;
        let Some(transaction) = &mut self.transaction else { return false };
        // A window gone, or one sent another size since: not waited for any more.
        transaction.waiting.retain(|key| waits.contains_key(key));
        if !due(transaction.waiting.len(), transaction.started, Instant::now()) {
            return false;
        }
        let transaction = self.transaction.take().unwrap();
        if !transaction.waiting.is_empty() {
            tracing::debug!(windows = transaction.waiting.len(), "layout shown without waiting for every window");
        }
        if transaction.animate {
            self.resize_animation = Some(crate::tiling::ResizeAnimation::from_rects(transaction.from));
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_resized_windows_with_a_size_to_draw_are_waited_for() {
        let r = |x, w| Rect::new(x, 0, w, 100);
        let from = HashMap::from([(1, r(0, 100)), (2, r(100, 100)), (3, r(200, 100)), (5, r(300, 100))]);
        // 1 only moves, 2 shrinks, 3 grows, 4 is new, 5 shrinks but already drew it.
        let to = [(1, r(10, 100)), (2, r(110, 50)), (3, r(160, 140)), (4, r(300, 100)), (5, r(300, 60))];
        let sent: HashMap<TileId, u32> = (1..=5).map(|t| (t, t * 10)).collect();
        assert_eq!(waiting_for(&from, &to, &sent, |key| *key != 50), HashSet::from([20, 30]));
    }

    #[test]
    fn it_ends_when_drawn_or_timed_out() {
        let start = Instant::now();
        assert!(due(0, start, start));
        assert!(!due(2, start, start + Duration::from_millis(50)));
        assert!(due(2, start, start + TIMEOUT));
    }
}
