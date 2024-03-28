use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

type UserId = String;

pub trait Clock: std::fmt::Debug + Send + Sync {
    fn now(&self) -> u64;
}

#[derive(Debug)]
pub struct StdTime {}

impl Clock for StdTime {
    fn now(&self) -> u64 {
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        since_the_epoch.as_secs()
    }
}

#[derive(Debug)]
pub struct DispenseTracker {
    tracked: HashMap<UserId, u64>,
    queue: BTreeMap<u64, Vec<UserId>>,
    in_progress: HashSet<UserId>,
    clock: Box<dyn Clock>,
}

impl Default for DispenseTracker {
    fn default() -> Self {
        Self {
            tracked: HashMap::default(),
            queue: Default::default(),
            in_progress: HashSet::default(),
            clock: Box::new(StdTime {}),
        }
    }
}

impl DispenseTracker {
    pub fn new(clock: impl Clock + 'static) -> Self {
        Self {
            tracked: HashMap::new(),
            queue: Default::default(),
            in_progress: HashSet::new(),
            clock: Box::new(clock),
        }
    }

    pub fn track(&mut self, user_id: String) {
        self.in_progress.remove(&user_id);

        let timestamp = self.clock.now();
        self.tracked.insert(user_id.clone(), timestamp);
        self.queue.entry(timestamp).or_default().push(user_id);
    }

    pub fn mark_in_progress(&mut self, user_id: String) {
        self.in_progress.insert(user_id);
    }

    pub fn remove_in_progress(&mut self, user_id: &String) {
        self.in_progress.remove(user_id);
    }

    pub fn evict_expired_entries(&mut self, eviction_duration: u64) {
        let now = self.clock.now();

        while let Some(oldest_entry) = self.queue.first_entry() {
            if now - oldest_entry.key() > eviction_duration {
                let (_, user_ids) = oldest_entry.remove_entry();

                for user_id in user_ids {
                    self.tracked.remove(&user_id);
                }
            } else {
                break;
            }
        }
    }

    pub fn has_tracked(&self, user_id: &UserId) -> bool {
        self.tracked.get(user_id).is_some()
    }

    pub fn is_in_progress(&self, user_id: &UserId) -> bool {
        self.in_progress.contains(user_id)
    }
}
