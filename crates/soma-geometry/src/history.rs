/// Bounded, transactional undo/redo history for cloneable editor state.
///
/// Apply edits to a private clone first. If the edit returns an error, the current
/// state and both history stacks remain unchanged. This snapshot-based foundation
/// is intentionally generic so an editor can store a combined mesh/selection state;
/// large scenes can later replace it with operation-specific deltas.
#[derive(Debug, Clone)]
pub struct EditHistory<T: Clone> {
    current: T,
    undo: Vec<T>,
    redo: Vec<T>,
    limit: usize,
}

impl<T: Clone> EditHistory<T> {
    /// Creates a history with a maximum number of undo snapshots.
    pub fn new(initial: T, limit: usize) -> Self {
        Self {
            current: initial,
            undo: Vec::new(),
            redo: Vec::new(),
            limit,
        }
    }

    /// Returns the current state without allowing untracked mutation.
    pub fn current(&self) -> &T {
        &self.current
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    /// Applies an edit atomically and records the prior state on success.
    ///
    /// Failed edits do not change the current state, undo history, or redo history.
    /// A successful edit clears redo history, as expected for a new branch of edits.
    pub fn apply<E>(
        &mut self,
        edit: impl FnOnce(&mut T) -> Result<(), E>,
    ) -> Result<(), E> {
        let mut candidate = self.current.clone();
        edit(&mut candidate)?;

        if self.limit > 0 {
            self.undo.push(std::mem::replace(&mut self.current, candidate));
            if self.undo.len() > self.limit {
                self.undo.remove(0);
            }
        } else {
            self.current = candidate;
        }
        self.redo.clear();
        Ok(())
    }

    /// Restores the previous state. Returns false when there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(&mut self.current, previous));
        true
    }

    /// Reapplies an undone state. Returns false when there is nothing to redo.
    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(&mut self.current, next));
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::EditHistory;

    #[test]
    fn undo_and_redo_restore_exact_snapshots() {
        let mut history = EditHistory::new(vec![1, 2], 10);
        history.apply(|state| { state.push(3); Ok::<_, ()>(()) }).unwrap();
        history.apply(|state| { state[0] = 9; Ok::<_, ()>(()) }).unwrap();
        assert_eq!(history.current(), &vec![9, 2, 3]);

        assert!(history.undo());
        assert_eq!(history.current(), &vec![1, 2, 3]);
        assert!(history.undo());
        assert_eq!(history.current(), &vec![1, 2]);
        assert!(!history.can_undo());
        assert!(history.redo());
        assert_eq!(history.current(), &vec![1, 2, 3]);
        assert!(history.redo());
        assert_eq!(history.current(), &vec![9, 2, 3]);
        assert!(!history.can_redo());
    }

    #[test]
    fn failed_edit_is_atomic_and_preserves_redo_history() {
        let mut history = EditHistory::new(vec![1], 5);
        history.apply(|state| { state.push(2); Ok::<_, &'static str>(()) }).unwrap();
        assert!(history.undo());

        let result = history.apply(|state| {
            state.push(99);
            Err::<(), _>("rejected")
        });
        assert_eq!(result, Err("rejected"));
        assert_eq!(history.current(), &vec![1]);
        assert!(history.can_redo());
        assert_eq!(history.redo_len(), 1);
    }

    #[test]
    fn new_edit_clears_redo_and_history_is_bounded() {
        let mut history = EditHistory::new(0, 2);
        for value in 1..=3 {
            history.apply(|state| { *state = value; Ok::<_, ()>(()) }).unwrap();
        }
        assert_eq!(history.undo_len(), 2);
        assert!(history.undo());
        assert_eq!(*history.current(), 2);
        history.apply(|state| { *state = 7; Ok::<_, ()>(()) }).unwrap();
        assert!(!history.can_redo());
        assert_eq!(*history.current(), 7);
    }

    #[test]
    fn zero_history_limit_keeps_edits_but_disables_undo() {
        let mut history = EditHistory::new(1, 0);
        history.apply(|state| { *state = 2; Ok::<_, ()>(()) }).unwrap();
        assert_eq!(*history.current(), 2);
        assert!(!history.can_undo());
        assert!(!history.undo());
    }
}
