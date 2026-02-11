use xt_core::model::Entry;

pub const DEFAULT_HISTORY_LIMIT: usize = 100;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SingleEditOp {
    pub index: usize,
    pub before_source: String,
    pub before_target: String,
    pub after_source: String,
    pub after_target: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchTargetChange {
    pub index: usize,
    pub before_target: String,
    pub after_target: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum EntryOp {
    SingleEdit(SingleEditOp),
    BatchTargetEdit(Vec<BatchTargetChange>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryHistory {
    past: Vec<EntryOp>,
    future: Vec<EntryOp>,
    limit: usize,
}

impl EntryHistory {
    pub fn with_limit(limit: usize) -> Self {
        Self {
            past: Vec::new(),
            future: Vec::new(),
            limit,
        }
    }

    pub fn clear(&mut self) {
        self.past.clear();
        self.future.clear();
    }

    pub fn record_single_edit(&mut self, op: SingleEditOp) -> bool {
        if op.before_source == op.after_source && op.before_target == op.after_target {
            return false;
        }
        self.push_op(EntryOp::SingleEdit(op));
        true
    }

    pub fn record_batch_target_edit(&mut self, mut changes: Vec<BatchTargetChange>) -> bool {
        changes.retain(|c| c.before_target != c.after_target);
        if changes.is_empty() {
            return false;
        }
        self.push_op(EntryOp::BatchTargetEdit(changes));
        true
    }

    pub fn undo(&mut self, entries: &mut [Entry]) -> bool {
        let Some(op) = self.past.pop() else {
            return false;
        };
        if !apply_op(entries, &op, false) {
            self.past.clear();
            self.future.clear();
            return false;
        }
        self.future.push(op);
        true
    }

    pub fn redo(&mut self, entries: &mut [Entry]) -> bool {
        let Some(op) = self.future.pop() else {
            return false;
        };
        if !apply_op(entries, &op, true) {
            self.past.clear();
            self.future.clear();
            return false;
        }
        self.past.push(op);
        true
    }

    fn push_op(&mut self, op: EntryOp) {
        self.past.push(op);
        if self.past.len() > self.limit {
            let overflow = self.past.len() - self.limit;
            self.past.drain(..overflow);
        }
        self.future.clear();
    }
}

fn apply_op(entries: &mut [Entry], op: &EntryOp, forward: bool) -> bool {
    match op {
        EntryOp::SingleEdit(op) => apply_single(entries, op, forward),
        EntryOp::BatchTargetEdit(changes) => apply_batch_target(entries, changes, forward),
    }
}

fn apply_single(entries: &mut [Entry], op: &SingleEditOp, forward: bool) -> bool {
    let Some(entry) = entries.get_mut(op.index) else {
        return false;
    };
    if forward {
        entry.source_text = op.after_source.clone();
        entry.target_text = op.after_target.clone();
    } else {
        entry.source_text = op.before_source.clone();
        entry.target_text = op.before_target.clone();
    }
    true
}

fn apply_batch_target(entries: &mut [Entry], changes: &[BatchTargetChange], forward: bool) -> bool {
    for change in changes {
        let Some(entry) = entries.get_mut(change.index) else {
            return false;
        };
        if forward {
            entry.target_text = change.after_target.clone();
        } else {
            entry.target_text = change.before_target.clone();
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, src: &str, dst: &str) -> Entry {
        Entry {
            key: key.to_string(),
            source_text: src.to_string(),
            target_text: dst.to_string(),
        }
    }

    #[test]
    fn t_hist_001_single_edit_undo_redo() {
        let mut hist = EntryHistory::with_limit(10);
        let mut entries = vec![entry("k1", "x", "y")];
        hist.record_single_edit(SingleEditOp {
            index: 0,
            before_source: "a".to_string(),
            before_target: "b".to_string(),
            after_source: "x".to_string(),
            after_target: "y".to_string(),
        });
        assert!(hist.undo(&mut entries));
        assert_eq!(entries[0].source_text, "a");
        assert_eq!(entries[0].target_text, "b");
        assert!(hist.redo(&mut entries));
        assert_eq!(entries[0].source_text, "x");
        assert_eq!(entries[0].target_text, "y");
    }

    #[test]
    fn t_hist_002_batch_target_undo_redo() {
        let mut hist = EntryHistory::with_limit(10);
        let mut entries = vec![entry("k1", "a", "1"), entry("k2", "b", "2")];
        hist.record_batch_target_edit(vec![
            BatchTargetChange {
                index: 0,
                before_target: "0".to_string(),
                after_target: "1".to_string(),
            },
            BatchTargetChange {
                index: 1,
                before_target: "0".to_string(),
                after_target: "2".to_string(),
            },
        ]);
        assert!(hist.undo(&mut entries));
        assert_eq!(entries[0].target_text, "0");
        assert_eq!(entries[1].target_text, "0");
        assert!(hist.redo(&mut entries));
        assert_eq!(entries[0].target_text, "1");
        assert_eq!(entries[1].target_text, "2");
    }

    #[test]
    fn t_hist_003_history_limit() {
        let mut hist = EntryHistory::with_limit(3);
        for i in 0..10usize {
            hist.record_batch_target_edit(vec![BatchTargetChange {
                index: 0,
                before_target: format!("{i}"),
                after_target: format!("{}", i + 1),
            }]);
        }
        let mut entries = vec![entry("k1", "a", "10")];
        let mut undo_count = 0usize;
        while hist.undo(&mut entries) {
            undo_count += 1;
        }
        assert_eq!(undo_count, 3);
    }
}
