// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The run recheck of repaired spans (P5 §7.2, step 5, substeps 2 to 4).
//!
//! The repaired bytes are written into a copy of the segment, which is delimited again in runs.
//! Each run is walked once and either checks or fails. A span that touches a failed run, or that
//! depends on a run that a removed span touches, is removed.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use super::{
    frame::{decode_frame, Frame, Storage},
    framing::item_end_examined,
    parity_repair::{Patch, Repair},
    payload::{PayloadBudget, PayloadFields, PayloadState},
    reader::WireSegment,
};

/// The envelope bytes that begin every parity frame (P5 6.5).
const PARITY_PREFIX: [u8; 2] = [0x8b, 0x02];

/// The positions of the spans that the run recheck rejects.
///
/// The candidate copy applies the data and parity patches of every repair, then checks each run.
/// A repair whose digests do not hold is rejected at once.
pub(super) fn rejected(
    data: &[u8],
    segment: &WireSegment<'_>,
    repairs: &[Repair],
    budget: &mut PayloadBudget,
) -> BTreeSet<usize> {
    let mut rejected: BTreeSet<usize> = repairs
        .iter()
        .filter(|repair| !repair.digests_hold)
        .map(|repair| repair.position)
        .collect();
    let (Some(opener), Some(last)) = (segment.items.first(), segment.items.last()) else {
        return rejected;
    };
    if repairs.iter().all(|repair| repair.data.is_empty()) {
        return rejected;
    }
    let first = opener.offset;
    let mut working = data[first..last.end].to_vec();
    let starts: Vec<usize> = segment
        .items
        .iter()
        .map(|item| item.offset - first)
        .collect();
    let touched = touched_items(&mut working, repairs, &starts, first);
    let mut boundaries: BTreeSet<usize> = starts.iter().copied().collect();
    boundaries.insert(working.len());
    let original_heads: HashMap<usize, [u8; 32]> = segment
        .frames
        .iter()
        .filter_map(|frame| Some((frame.position, frame.link.effective_identifier?)))
        .collect();
    let (registry, identifier) = segment.opener.state();
    let Some(h0) = identifier else {
        return rejected;
    };
    let mut walk = Walk {
        source: &data[first..last.end],
        working: &working,
        boundaries: &boundaries,
        opener_end: opener.end - first,
        registry,
        budget,
    };
    let mut state = Chain {
        predecessor: h0,
        predecessor_run: None,
        failed_since_predecessor: false,
    };
    let mut dependents: HashMap<usize, BTreeSet<usize>> = HashMap::new();
    let mut runs_by_span: HashMap<usize, BTreeSet<usize>> = HashMap::new();
    let mut run_count = 0;
    let mut covered_until = 0;
    for (index, &cursor) in starts.iter().enumerate() {
        if index < covered_until {
            continue;
        }
        if !touched.contains_key(&index) {
            if let Some(identifier) = original_heads.get(&index) {
                state.predecessor = *identifier;
                state.predecessor_run = None;
                state.failed_since_predecessor = false;
            }
            continue;
        }
        let run = walk.run(cursor, &state);
        covered_until = index + 1 + starts[index + 1..].partition_point(|&s| s < run.examined_end);
        let participants: BTreeSet<usize> = (index..covered_until)
            .filter_map(|position| touched.get(&position))
            .flatten()
            .copied()
            .collect();
        for span in &participants {
            runs_by_span.entry(*span).or_default().insert(run_count);
        }
        dependents
            .entry(run_count)
            .or_default()
            .extend(participants.iter().copied());
        if let Some(previous) = state.predecessor_run {
            dependents
                .entry(previous)
                .or_default()
                .extend(participants.iter().copied());
        }
        if !run.holds || state.failed_since_predecessor {
            rejected.extend(participants.iter().copied());
        }
        if !run.holds {
            state.failed_since_predecessor = true;
        } else if let Some(last_ordinary) = run.last_ordinary {
            state.predecessor = last_ordinary;
            state.predecessor_run = Some(run_count);
            state.failed_since_predecessor = false;
        }
        run_count += 1;
    }
    propagate(&mut rejected, &runs_by_span, &mut dependents);
    rejected
}

/// The chain state of the walks: the predecessor of the next frame and the run that gives it.
struct Chain {
    predecessor: [u8; 32],
    predecessor_run: Option<usize>,
    failed_since_predecessor: bool,
}

/// The outcome of one walked run.
struct RunOutcome {
    examined_end: usize,
    holds: bool,
    last_ordinary: Option<[u8; 32]>,
}

struct Walk<'a, 'b> {
    source: &'a [u8],
    working: &'a [u8],
    boundaries: &'a BTreeSet<usize>,
    opener_end: usize,
    registry: u64,
    budget: &'b mut PayloadBudget,
}

impl Walk<'_, '_> {
    /// Walk the items of the copy from `start` to the next input boundary (P5 7.2, step 5).
    fn run(&mut self, start: usize, state: &Chain) -> RunOutcome {
        let mut cursor = start;
        let mut predecessor = state.predecessor;
        let mut last_ordinary = None;
        loop {
            let failed = RunOutcome {
                examined_end: 0,
                holds: false,
                last_ordinary: None,
            };
            let end = match item_end_examined(self.working, cursor) {
                Ok(end) => end,
                Err(failure) => {
                    return RunOutcome {
                        examined_end: failure.examined_end,
                        ..failed
                    };
                }
            };
            let Some(ordinary) = self.item_checks(cursor, end, &mut predecessor) else {
                return RunOutcome {
                    examined_end: end,
                    ..failed
                };
            };
            if ordinary {
                last_ordinary = Some(predecessor);
            }
            if self.boundaries.contains(&end) {
                return RunOutcome {
                    examined_end: end,
                    holds: true,
                    last_ordinary,
                };
            }
            cursor = end;
        }
    }

    /// Check one walked item: the opener by its bytes, a frame by its envelope, its identifier and
    /// its payload (P5 7.2, step 5, substep 3). Returns whether the item is an ordinary frame, or
    /// `None` when the item fails; advances `predecessor` past an ordinary frame.
    fn item_checks(
        &mut self,
        cursor: usize,
        end: usize,
        predecessor: &mut [u8; 32],
    ) -> Option<bool> {
        if cursor == 0 {
            let opener_passes = end == self.opener_end && self.working[..end] == self.source[..end];
            return opener_passes.then_some(false);
        }
        if self.working.get(cursor..cursor + PARITY_PREFIX.len()) != Some(&PARITY_PREFIX) {
            return None;
        }
        let frame = decode_frame(self.working, cursor, self.registry).ok()?;
        if frame.computed_identifier(predecessor) != *frame.identifier
            || !recheck(self.budget, &frame)
        {
            return None;
        }
        if frame.transparent {
            return Some(false);
        }
        *predecessor = *frame.identifier;
        Some(true)
    }
}

/// The payload recheck of a walked frame: a matching stored check, and a verified payload or, for
/// a compressed frame, an unchecked one (P5 7.2, step 5, substep 3).
fn recheck(budget: &mut PayloadBudget, frame: &Frame<'_>) -> bool {
    let stored = frame.check_storage();
    if frame.storage != Storage::Plain && stored != Some(true) {
        return false;
    }
    let check = budget.check(&PayloadFields::from(frame), stored);
    matches!(check.state, PayloadState::Verified(_))
        || (matches!(frame.storage, Storage::Compressed { .. })
            && matches!(check.state, PayloadState::Unchecked(_)))
}

/// Apply the patches of every repair to the copy, and map each data patch to the item it lies in.
fn touched_items(
    working: &mut [u8],
    repairs: &[Repair],
    starts: &[usize],
    first: usize,
) -> BTreeMap<usize, BTreeSet<usize>> {
    let mut touched: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for repair in repairs {
        for patch in &repair.data {
            apply(working, patch, first);
            let item = starts
                .partition_point(|&start| start <= patch.offset - first)
                .saturating_sub(1);
            touched.entry(item).or_default().insert(repair.position);
        }
        for patch in &repair.parity {
            apply(working, patch, first);
        }
    }
    touched
}

fn apply(working: &mut [u8], patch: &Patch, first: usize) {
    let start = patch.offset - first;
    working[start..start + patch.data.len()].copy_from_slice(&patch.data);
}

/// Remove every span that depends on a run that a removed span touches, until none is left.
fn propagate(
    rejected: &mut BTreeSet<usize>,
    runs_by_span: &HashMap<usize, BTreeSet<usize>>,
    dependents: &mut HashMap<usize, BTreeSet<usize>>,
) {
    let mut pending: VecDeque<usize> = rejected.iter().copied().collect();
    while let Some(span) = pending.pop_front() {
        let Some(runs) = runs_by_span.get(&span) else {
            continue;
        };
        for run in runs {
            let Some(spans) = dependents.remove(run) else {
                continue;
            };
            for dependent in spans {
                if rejected.insert(dependent) {
                    pending.push_back(dependent);
                }
            }
        }
    }
}
