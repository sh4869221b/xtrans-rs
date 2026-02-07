#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoStack<T> {
    past: Vec<T>,
    present: T,
    future: Vec<T>,
}

impl<T: Clone + PartialEq> UndoStack<T> {
    pub fn new(initial: T) -> Self {
        Self {
            past: Vec::new(),
            present: initial,
            future: Vec::new(),
        }
    }

    pub fn present(&self) -> &T {
        &self.present
    }

    pub fn apply(&mut self, next: T) {
        if next == self.present {
            return;
        }
        self.past.push(self.present.clone());
        self.present = next;
        self.future.clear();
    }

    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.past.pop() {
            self.future.push(self.present.clone());
            self.present = prev;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.future.pop() {
            self.past.push(self.present.clone());
            self.present = next;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_undo_001_single_edit_undo_redo() {
        let mut stack = UndoStack::new("a".to_string());
        stack.apply("b".to_string());
        assert_eq!(stack.present(), "b");
        assert!(stack.undo());
        assert_eq!(stack.present(), "a");
        assert!(stack.redo());
        assert_eq!(stack.present(), "b");
    }

    #[test]
    fn t_undo_002_batch_edit_undo() {
        let mut stack = UndoStack::new(vec!["a".to_string(), "b".to_string()]);
        let mut next = stack.present().clone();
        next[0] = "x".to_string();
        next[1] = "y".to_string();
        stack.apply(next);
        assert_eq!(stack.present(), &vec!["x".to_string(), "y".to_string()]);
        assert!(stack.undo());
        assert_eq!(stack.present(), &vec!["a".to_string(), "b".to_string()]);
    }
}
