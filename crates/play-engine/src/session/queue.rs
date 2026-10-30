//! FIFO per Exploration, round-robin between eligible Explorations.
use std::collections::VecDeque;
use uuid::Uuid;

pub(super) struct Queue<T> {
    lanes: VecDeque<(Uuid, VecDeque<T>)>,
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        Self {
            lanes: VecDeque::new(),
        }
    }
}

impl<T> Queue<T> {
    pub fn len(&self) -> usize {
        self.lanes.iter().map(|(_, entries)| entries.len()).sum()
    }

    pub fn count(&self, owner: Uuid) -> usize {
        self.lanes
            .iter()
            .find(|(id, _)| *id == owner)
            .map_or(0, |(_, entries)| entries.len())
    }

    pub fn push(&mut self, owner: Uuid, item: T) {
        if let Some((_, entries)) = self.lanes.iter_mut().find(|(id, _)| *id == owner) {
            entries.push_back(item);
        } else {
            self.lanes.push_back((owner, VecDeque::from([item])));
        }
    }

    pub fn pop(&mut self, eligible: impl Fn(Uuid) -> bool) -> Option<T> {
        for _ in 0..self.lanes.len() {
            let (owner, mut entries) = self.lanes.pop_front()?;
            let item = eligible(owner).then(|| entries.pop_front()).flatten();
            if !entries.is_empty() {
                self.lanes.push_back((owner, entries));
            }
            if item.is_some() {
                return item;
            }
        }
        None
    }

    pub fn remove(&mut self, matches: impl Fn(&T) -> bool) -> Option<T> {
        for (_, entries) in &mut self.lanes {
            if let Some(index) = entries.iter().position(&matches) {
                let item = entries.remove(index);
                self.lanes.retain(|(_, entries)| !entries.is_empty());
                return item;
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_backlogged_exploration_cannot_starve_other_eligible_lanes() {
        let [a, b, c] = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
        let mut queue = Queue::default();
        for (owner, value) in [(a, 1), (a, 2), (b, 3), (b, 4), (c, 5), (c, 6)] {
            queue.push(owner, value);
        }
        assert_eq!(queue.len(), 6);
        assert_eq!(queue.count(a), 2);
        assert_eq!(queue.pop(|_| true), Some(1));
        queue.push(a, 7);
        assert_eq!(queue.pop(|_| true), Some(3));
        queue.push(b, 8);
        assert_eq!(queue.pop(|_| true), Some(5));
        assert_eq!(queue.pop(|_| true), Some(2));
        assert_eq!(queue.pop(|_| true), Some(4));
        assert_eq!(queue.pop(|_| true), Some(6));
        assert_eq!(queue.pop(|_| true), Some(7));
        assert_eq!(queue.pop(|_| true), Some(8));
        assert_eq!(queue.pop(|_| true), None);
    }

    #[test]
    fn busy_lanes_keep_fifo_and_cancellation_does_not_reorder_others() {
        let [a, b, c] = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
        let mut queue = Queue::default();
        queue.push(a, 1);
        queue.push(a, 2);
        queue.push(b, 3);
        queue.push(c, 4);
        assert_eq!(queue.pop(|_| false), None);
        assert_eq!(queue.pop(|owner| owner != a), Some(3));
        assert_eq!(queue.remove(|value| *value == 1), Some(1));
        assert_eq!(queue.remove(|value| *value == 99), None);
        assert_eq!(queue.pop(|_| true), Some(4));
        assert_eq!(queue.pop(|_| true), Some(2));
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.count(a), 0);
    }
}
