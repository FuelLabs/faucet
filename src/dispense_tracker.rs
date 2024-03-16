use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use fuel_types::Address;

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
    tracked: HashMap<Address, u64>,
    queue: BTreeMap<u64, Vec<Address>>,
    in_progress: HashSet<Address>,
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

    pub fn track(&mut self, address: Address) {
        self.in_progress.remove(&address);

        let timestamp = self.clock.now();
        self.tracked.insert(address, timestamp);
        self.queue.entry(timestamp).or_default().push(address);
    }

    pub fn mark_in_progress(&mut self, address: Address) {
        self.in_progress.insert(address);
    }

    pub fn remove_in_progress(&mut self, address: &Address) {
        self.in_progress.remove(address);
    }

    pub fn evict_expired_entries(&mut self, eviction_duration: u64) {
        let now = self.clock.now();

        while let Some(oldest_entry) = self.queue.first_entry() {
            if now - oldest_entry.key() > eviction_duration {
                let (_, addresses) = oldest_entry.remove_entry();

                for address in addresses {
                    self.tracked.remove(&address);
                }
            } else {
                break;
            }
        }
    }

    pub fn has_tracked(&self, address: &Address) -> bool {
        self.tracked.get(address).is_some()
    }

    pub fn is_in_progress(&self, address: &Address) -> bool {
        self.in_progress.contains(address)
    }
}
