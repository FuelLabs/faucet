use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
};

type UserId = String;

#[derive(Debug, Eq, PartialEq)]
pub struct Entry {
    user_id: UserId,
    timestamp: u64,
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub trait Clock: std::fmt::Debug + Send + Sync {
    fn now(&self) -> u64;
}

#[derive(Debug)]
pub struct TokioTime {}

impl Clock for TokioTime {
    fn now(&self) -> u64 {
        tokio::time::Instant::now().elapsed().as_secs()
    }
}

#[derive(Debug)]
pub struct DispenseTracker {
    tracked: HashMap<UserId, u64>,
    queue: BinaryHeap<Entry>,
    in_progress: HashSet<UserId>,
    clock: Box<dyn Clock>,
}

impl Default for DispenseTracker {
    fn default() -> Self {
        Self {
            tracked: HashMap::default(),
            queue: BinaryHeap::default(),
            in_progress: HashSet::default(),
            clock: Box::new(TokioTime {}),
        }
    }
}

impl DispenseTracker {
    pub fn new(clock: impl Clock + 'static) -> Self {
        Self {
            tracked: HashMap::new(),
            queue: BinaryHeap::new(),
            in_progress: HashSet::new(),
            clock: Box::new(clock),
        }
    }

    pub fn track(&mut self, user_id: String) {
        self.in_progress.remove(&user_id);

        let timestamp = self.clock.now();
        self.tracked.insert(user_id.clone(), timestamp);
        self.queue.push(Entry { user_id, timestamp });
    }

    pub fn mark_in_progress(&mut self, user_id: String) {
        self.in_progress.insert(user_id);
    }

    pub fn remove_in_progress(&mut self, user_id: &String) {
        self.in_progress.remove(user_id);
    }

    pub fn evict_expired_entries(&mut self, eviction_duration: u64) {
        let now = self.clock.now();

        while let Some(oldest_entry) = self.queue.peek() {
            if now - oldest_entry.timestamp > eviction_duration {
                let removed_entry = self.queue.pop().unwrap();
                self.tracked.remove(&removed_entry.user_id);
            } else {
                break;
            }
        }
    }

    pub fn has_tracked(&self, user_id: &String) -> bool {
        self.tracked.get(user_id).is_some() || self.in_progress.contains(user_id)
    }
}
