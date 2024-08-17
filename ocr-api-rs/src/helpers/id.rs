use std::{
    hash::{Hash, Hasher},
    process, thread, time,
};

use super::radix_fmt::{FormatRadix, MAX_BASE};

fn now_ns() -> u128 {
    time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_nanos()
}

fn thread_id_hash() -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    thread::current().id().hash(&mut hasher);
    hasher.finish()
}

#[must_use]
pub fn time_thread_id() -> String {
    let thread_id = thread_id_hash().format_to_base(MAX_BASE);
    let process_id = process::id().format_to_base(MAX_BASE);
    let ns = now_ns().format_to_base(MAX_BASE);

    let id = format!("{ns}-{process_id}-{thread_id}");

    id
}
