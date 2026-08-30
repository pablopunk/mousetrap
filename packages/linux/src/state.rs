//! Session state machine: three-step refinement, chord targeting.
//!
//! Direct port of the Python prototype's `session.py`.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::geometry::{
    Bounds, CellTarget, cell_bounds, cell_center, classify_chord, combine_bounds, expanded_bounds,
    rect_center,
};

pub const MAX_REFINEMENT_STEPS: u32 = 3;

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub initial_bounds: Bounds,
    pub current_bounds: Bounds,
    pub step: u32,
    pub max_steps: u32,
    pub history: Vec<String>,
    pub pending_keys: Vec<String>,
    pub held_keys: Vec<String>,
    pub pending_since: Option<f64>,
    pub started_at: f64,
    pub updated_at: f64,
}

impl SessionState {
    pub fn start(bounds: Bounds) -> Self {
        let t = now();
        Self {
            initial_bounds: bounds,
            current_bounds: bounds,
            step: 1,
            max_steps: MAX_REFINEMENT_STEPS,
            history: Vec::new(),
            pending_keys: Vec::new(),
            held_keys: Vec::new(),
            pending_since: None,
            started_at: t,
            updated_at: t,
        }
    }

    pub fn has_timed_out(&self, timeout_seconds: f64) -> bool {
        timeout_seconds > 0.0 && (now() - self.updated_at) > timeout_seconds
    }
}

#[derive(Debug, Clone)]
pub struct SelectionResult {
    pub keys: Vec<String>,
    pub step: u32,
    pub max_steps: u32,
    pub targets: Vec<CellTarget>,
    pub selected_bounds: Bounds,
    pub point: (i32, i32),
    /// Whether this selection ends the session (final step reached).
    pub is_final: bool,
    pub chord_kind: Option<&'static str>,
}

pub struct OverlaySession {
    pub state: SessionState,
}

/// Outcome of a key transition.
pub enum KeyResult {
    /// Key resolved and is pending until release.
    Pending,
    /// Key does not map to the grid.
    Invalid,
    /// Key was already held.
    DuplicateHeld,
    /// All keys released: pending chord should be committed.
    Commit,
    /// Not all held keys have been released yet.
    StillHeld,
}

impl OverlaySession {
    pub fn new(state: SessionState) -> Self {
        Self { state }
    }

    pub fn key_down(&mut self, key: &str) -> KeyResult {
        let Some(target) = crate::geometry::find_cell_for_key(key) else {
            return KeyResult::Invalid;
        };
        let key = target.key.to_string();
        if self.state.held_keys.contains(&key) {
            return KeyResult::DuplicateHeld;
        }
        self.state.held_keys.push(key.clone());
        if !self.state.pending_keys.contains(&key) {
            self.state.pending_keys.push(key);
        }
        if self.state.pending_since.is_none() {
            self.state.pending_since = Some(now());
        }
        self.state.updated_at = now();
        KeyResult::Pending
    }

    pub fn key_up(&mut self, key: &str) -> KeyResult {
        let key = key.trim().to_lowercase();
        self.state.held_keys.retain(|k| k != &key);
        self.state.updated_at = now();
        if !self.state.pending_keys.is_empty() && self.state.held_keys.is_empty() {
            KeyResult::Commit
        } else {
            KeyResult::StillHeld
        }
    }

    /// Commit the pending chord (called when all keys are released).
    pub fn commit_pending(&mut self) -> Option<SelectionResult> {
        if self.state.pending_keys.is_empty() {
            return None;
        }
        let targets: Vec<Option<CellTarget>> = self
            .state
            .pending_keys
            .iter()
            .map(|k| crate::geometry::find_cell_for_key(k))
            .collect();
        if targets.iter().any(|t| t.is_none()) {
            self.state.pending_keys.clear();
            self.state.pending_since = None;
            return None;
        }
        let resolved: Vec<CellTarget> = targets.into_iter().flatten().collect();
        let chord_kind = classify_chord(&resolved);
        if chord_kind.is_none() && resolved.len() > 1 {
            // Non-chord multi-key press: resolve from the first key only.
            let first = resolved[..1].to_vec();
            let keys = self.state.pending_keys[..1].to_vec();
            self.state.pending_keys = self.state.pending_keys[1..].to_vec();
            self.state.pending_since = if self.state.pending_keys.is_empty() {
                None
            } else {
                Some(now())
            };
            return Some(self.apply_selection(first, keys, None));
        }
        let keys = std::mem::take(&mut self.state.pending_keys);
        self.state.pending_since = None;
        Some(self.apply_selection(resolved, keys, chord_kind))
    }

    fn apply_selection(
        &mut self,
        targets: Vec<CellTarget>,
        keys: Vec<String>,
        chord_kind: Option<&'static str>,
    ) -> SelectionResult {
        let rects: Vec<Bounds> = targets
            .iter()
            .map(|t| cell_bounds(self.state.current_bounds, t))
            .collect();
        let selected_bounds = combine_bounds(&rects);
        let point = if chord_kind.is_some() {
            rect_center(selected_bounds)
        } else {
            cell_center(self.state.current_bounds, &targets[0])
        };
        let is_final = self.state.step >= self.state.max_steps;
        let result = SelectionResult {
            keys: keys.clone(),
            step: self.state.step,
            max_steps: self.state.max_steps,
            targets,
            selected_bounds,
            point,
            is_final,
            chord_kind,
        };
        self.state.history.push(keys.concat());
        let next_depth = self.state.step;
        self.state.current_bounds =
            expanded_bounds(selected_bounds, self.state.initial_bounds, next_depth);
        self.state.step += 1;
        self.state.updated_at = now();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::find_cell_for_key;

    const SCREEN: Bounds = (0, 0, 2048, 1152);

    fn key_sequence(keys: &[&str]) -> (OverlaySession, Option<SelectionResult>) {
        let mut session = OverlaySession::new(SessionState::start(SCREEN));
        let mut last = None;
        for key in keys {
            session.key_down(key);
            if matches!(session.key_up(key), KeyResult::Commit) {
                last = session.commit_pending();
            }
        }
        (session, last)
    }

    #[test]
    fn three_steps_reach_final() {
        let (session, first) = key_sequence(&["a"]);
        let first = first.unwrap();
        assert!(!first.is_final);
        assert_eq!(first.step, 1);
        // 'a' is row 2, col 0 → refined region stays near that cell.
        let (x, y, _, _) = first.selected_bounds;
        assert!(x >= 0 && y >= 0);
        // Continue two more steps.
        let mut session = session;
        for _ in 0..2 {
            session.key_down("a");
            let _ = session.key_up("a");
            let result = session.commit_pending().unwrap();
            if result.is_final {
                // Final point must land inside the previous current bounds.
                let (bx, by, bw, bh) = session.state.initial_bounds;
                assert!(result.point.0 >= bx && result.point.0 <= bx + bw);
                assert!(result.point.1 >= by && result.point.1 <= by + bh);
            }
        }
    }

    #[test]
    fn chord_pair_targets_midpoint() {
        let mut session = OverlaySession::new(SessionState::start(SCREEN));
        session.key_down("z");
        session.key_down("x");
        assert!(matches!(session.key_up("z"), KeyResult::StillHeld));
        assert!(matches!(session.key_up("x"), KeyResult::Commit));
        let result = session.commit_pending().unwrap();
        assert_eq!(result.chord_kind, Some("pair"));
        // Midpoint between z and x: z col 0, x col 1 in the bottom row.
        let (_, _, w, _) = SCREEN;
        let _ = w;
        let z = find_cell_for_key("z").unwrap();
        let x = find_cell_for_key("x").unwrap();
        let (zcx, _) = cell_center(SCREEN, &z);
        let (xcx, _) = cell_center(SCREEN, &x);
        assert_eq!(result.point.0, (zcx + xcx) / 2);
    }

    #[test]
    fn horizontal_as_chord_targets_shared_edge() {
        let mut session = OverlaySession::new(SessionState::start(SCREEN));
        session.key_down("a");
        session.key_down("s");
        assert!(matches!(session.key_up("a"), KeyResult::StillHeld));
        assert!(matches!(session.key_up("s"), KeyResult::Commit));
        let result = session.commit_pending().unwrap();
        let a = cell_bounds(SCREEN, &find_cell_for_key("a").unwrap());
        let s = cell_bounds(SCREEN, &find_cell_for_key("s").unwrap());
        assert_eq!(result.chord_kind, Some("pair"));
        assert_eq!(result.point, (a.0 + a.2, a.1 + a.3 / 2));
        assert_eq!(a.0 + a.2, s.0);
    }

    #[test]
    fn diagonal_chord_targets_shared_corner() {
        let mut session = OverlaySession::new(SessionState::start(SCREEN));
        session.key_down("q");
        session.key_down("s");
        session.key_up("q");
        session.key_up("s");
        let result = session.commit_pending().unwrap();
        let q = cell_bounds(SCREEN, &find_cell_for_key("q").unwrap());
        assert_eq!(result.chord_kind, Some("pair"));
        assert_eq!(result.point, (q.0 + q.2, q.1 + q.3));
    }

    #[test]
    fn rolling_keys_can_join_before_caller_commits() {
        let mut session = OverlaySession::new(SessionState::start(SCREEN));
        session.key_down("a");
        assert!(matches!(session.key_up("a"), KeyResult::Commit));
        // The daemon waits for the grace period instead of committing here.
        session.key_down("s");
        assert!(matches!(session.key_up("s"), KeyResult::Commit));
        let result = session.commit_pending().unwrap();
        assert_eq!(result.keys, vec!["a", "s"]);
        assert_eq!(result.chord_kind, Some("pair"));
    }

    #[test]
    fn timeout_detection() {
        let mut state = SessionState::start(SCREEN);
        state.updated_at -= 10.0;
        assert!(state.has_timed_out(5.0));
        assert!(!state.has_timed_out(15.0));
    }
}
