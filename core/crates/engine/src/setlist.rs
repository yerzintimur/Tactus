//! Set lists: the module's own ordering of kits — 32 lists of 32 steps, each step
//! holding a kit number, terminated by [`END`] (MIDI Implementation §3, footnote
//! *2). This is how a drummer arranges kits for a gig, and on the V31 it is
//! otherwise only reachable through the screen.
//!
//! This module is the pure part: the cached list and the edits as *step writes*.
//! It performs no I/O and never guesses — a slot the module hasn't told us about
//! stays `None`, and an edit that would depend on one is refused rather than
//! computed from a guess.

use std::collections::VecDeque;

/// The raw value of the step that ends a set list ("END" on the module).
pub(crate) const END: i64 = -1;

/// One step write, still to be sent. Writes are queued and sent **one at a time**
/// (the next goes out when the module confirms the previous), so a reorder can
/// never leave the list half-written by a burst the module dropped, and a failure
/// stops the sequence instead of scrambling the order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StepWrite {
    pub step: u32,
    pub raw: i64,
}

/// The set list currently loaded for viewing and editing.
#[derive(Debug, Clone)]
pub(crate) struct SetlistState {
    pub index: u32,
    /// `None` until the module has told us.
    pub name: Option<String>,
    /// One slot per step the module has; `None` until read back.
    pub steps: Vec<Option<i64>>,
    pub queue: VecDeque<StepWrite>,
}

impl SetlistState {
    pub fn new(index: u32, capacity: usize) -> Self {
        Self {
            index,
            name: None,
            steps: vec![None; capacity],
            queue: VecDeque::new(),
        }
    }

    /// How many steps the list holds: everything before the first `END`. A slot we
    /// haven't read yet ends the count too — we report what the module confirmed,
    /// never a guess (ADR-0010).
    pub fn length(&self) -> usize {
        self.steps
            .iter()
            .position(|slot| !matches!(slot, Some(kit) if *kit != END))
            .unwrap_or(self.steps.len())
    }

    /// The kit at `step`, if the module has told us and it isn't the terminator.
    pub fn kit_at(&self, step: usize) -> Option<i64> {
        match self.steps.get(step) {
            Some(Some(kit)) if *kit != END => Some(*kit),
            _ => None,
        }
    }

    /// Add a kit to the end of the list, keeping it terminated.
    pub fn append(&self, kit: i64) -> Option<Vec<StepWrite>> {
        let at = self.length();
        if at >= self.steps.len() {
            return None; // the list is full
        }
        let mut writes = vec![StepWrite {
            step: at as u32,
            raw: kit,
        }];
        // The slot after the new last step becomes the terminator — unless the
        // kit just filled the final slot, where the list ends by running out.
        if at + 1 < self.steps.len() {
            writes.push(StepWrite {
                step: at as u32 + 1,
                raw: END,
            });
        }
        Some(writes)
    }

    /// Drop a step, shifting the rest up and re-terminating the list.
    pub fn remove(&self, step: usize) -> Option<Vec<StepWrite>> {
        let len = self.length();
        if step >= len {
            return None;
        }
        let mut writes = Vec::with_capacity(len - step);
        for i in step..len - 1 {
            writes.push(StepWrite {
                step: i as u32,
                raw: (*self.steps.get(i + 1)?)?,
            });
        }
        writes.push(StepWrite {
            step: (len - 1) as u32,
            raw: END,
        });
        Some(writes)
    }

    /// Exchange two steps — what "move up" / "move down" is built from, the
    /// reordering gesture a screen-reader user can actually perform.
    pub fn swap(&self, a: usize, b: usize) -> Option<Vec<StepWrite>> {
        let len = self.length();
        if a >= len || b >= len || a == b {
            return None;
        }
        let (first, second) = ((*self.steps.get(a)?)?, (*self.steps.get(b)?)?);
        Some(vec![
            StepWrite {
                step: a as u32,
                raw: second,
            },
            StepWrite {
                step: b as u32,
                raw: first,
            },
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A list of `kits` followed by the terminator, in a 6-slot module.
    fn loaded(kits: &[i64]) -> SetlistState {
        let mut s = SetlistState::new(0, 6);
        for (i, kit) in kits.iter().enumerate() {
            s.steps[i] = Some(*kit);
        }
        for slot in s.steps.iter_mut().skip(kits.len()) {
            *slot = Some(END);
        }
        s
    }

    #[test]
    fn length_stops_at_the_terminator() {
        assert_eq!(loaded(&[4, 9, 12]).length(), 3);
        assert_eq!(loaded(&[]).length(), 0);
        // A list that fills every slot has no terminator to find.
        assert_eq!(loaded(&[1, 2, 3, 4, 5, 6]).length(), 6);
    }

    #[test]
    fn length_never_counts_past_what_the_module_told_us() {
        // Mid-load: step 2 hasn't arrived, so the list is one step long so far —
        // reporting three would be inventing state we don't have.
        let mut s = SetlistState::new(0, 6);
        s.steps[0] = Some(4);
        s.steps[2] = Some(12);
        assert_eq!(s.length(), 1);
    }

    #[test]
    fn append_writes_the_kit_and_moves_the_terminator() {
        assert_eq!(
            loaded(&[4, 9]).append(12),
            Some(vec![
                StepWrite { step: 2, raw: 12 },
                StepWrite { step: 3, raw: END },
            ])
        );
    }

    #[test]
    fn append_into_the_last_slot_needs_no_terminator() {
        assert_eq!(
            loaded(&[1, 2, 3, 4, 5]).append(6),
            Some(vec![StepWrite { step: 5, raw: 6 }])
        );
        // …and a full list refuses rather than overwriting the last step.
        assert_eq!(loaded(&[1, 2, 3, 4, 5, 6]).append(7), None);
    }

    #[test]
    fn remove_shifts_the_rest_up_and_re_terminates() {
        assert_eq!(
            loaded(&[4, 9, 12]).remove(0),
            Some(vec![
                StepWrite { step: 0, raw: 9 },
                StepWrite { step: 1, raw: 12 },
                StepWrite { step: 2, raw: END },
            ])
        );
        // Removing the last step only moves the terminator.
        assert_eq!(
            loaded(&[4, 9, 12]).remove(2),
            Some(vec![StepWrite { step: 2, raw: END }])
        );
        // A step past the end of the list isn't a step.
        assert_eq!(loaded(&[4, 9, 12]).remove(3), None);
    }

    #[test]
    fn swap_exchanges_two_steps() {
        assert_eq!(
            loaded(&[4, 9, 12]).swap(0, 1),
            Some(vec![
                StepWrite { step: 0, raw: 9 },
                StepWrite { step: 1, raw: 4 },
            ])
        );
        assert_eq!(loaded(&[4, 9, 12]).swap(1, 1), None);
        assert_eq!(loaded(&[4, 9, 12]).swap(1, 3), None);
    }

    #[test]
    fn edits_refuse_to_run_on_a_half_read_list() {
        // Only step 0 has arrived: the list reads as one step long, so nothing
        // beyond it can be swapped or removed on values we don't have.
        let mut s = SetlistState::new(0, 6);
        s.steps[0] = Some(4);
        assert_eq!(s.swap(0, 1), None);
        assert_eq!(s.remove(1), None);
        assert_eq!(s.kit_at(1), None);
    }
}
