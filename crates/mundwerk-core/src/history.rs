//! The last dictations, newest first, so that one that landed in the wrong field can be pasted again or copied.

use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// When, as the app formats it (local time).
    pub when: String,
    pub text: String,
}

#[derive(Debug)]
pub struct History {
    size: usize,
    entries: VecDeque<Entry>,
}

impl History {
    #[must_use]
    pub fn new(size: usize) -> Self {
        Self { size, entries: VecDeque::with_capacity(size) }
    }

    pub fn add(&mut self, entry: Entry) {
        if self.size == 0 {
            return;
        }
        if self.entries.len() == self.size {
            self.entries.pop_back();
        }
        self.entries.push_front(entry);
    }

    /// Newest first.
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter()
    }

    /// A new size from the settings; the oldest entries beyond it go.
    pub fn resize(&mut self, size: usize) {
        self.size = size;
        self.entries.truncate(size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(text: &str) -> Entry {
        Entry { when: "12:00".into(), text: text.into() }
    }

    #[test]
    fn newest_first_and_bounded() {
        let mut history = History::new(2);
        for text in ["a", "b", "c"] {
            history.add(entry(text));
        }
        let texts: Vec<&str> = history.entries().map(|entry| entry.text.as_str()).collect();
        assert_eq!(texts, ["c", "b"]);
    }

    #[test]
    fn size_zero_keeps_nothing() {
        let mut history = History::new(0);
        history.add(entry("a"));
        assert_eq!(history.entries().count(), 0);
    }

    #[test]
    fn a_smaller_size_drops_the_oldest() {
        let mut history = History::new(3);
        for text in ["a", "b", "c"] {
            history.add(entry(text));
        }
        history.resize(1);
        assert_eq!(history.entries().next().unwrap().text, "c");
        assert_eq!(history.entries().count(), 1);
    }
}
