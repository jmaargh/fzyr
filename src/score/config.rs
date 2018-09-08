extern crate std;

use std::f64;

use score::Score;

/// Score given when the query doesn't match the candidate at all
pub const SCORE_MIN: Score = f64::NEG_INFINITY;
/// Score given when the query matches the candidate perfectly
pub const SCORE_MAX: Score = f64::INFINITY;

pub const SCORE_GAP_LEADING: Score = -0.005;
pub const SCORE_GAP_INNER: Score = -0.01;
pub const SCORE_GAP_TRAILING: Score = -0.005;

pub const SCORE_MATCH_CONSECUTIVE: Score = 1.0;
pub const SCORE_MATCH_SLASH: Score = 0.9;
pub const SCORE_MATCH_WORD: Score = 0.8;
pub const SCORE_MATCH_CAPITAL: Score = 0.7;
pub const SCORE_MATCH_DOT: Score = 0.6;

pub const CANDIDATE_MAX_BYTES: usize = 2048;
pub const CANDIDATE_MAX_CHARS: usize = 1024;

#[cfg(test)]
mod tests {
  use super::*;

  fn assert_positive(val: f64) {
    assert!(val > 0.0);
  }

  fn assert_negative(val: f64) {
    assert!(val < 0.0);
  }

  #[test]
  fn positive_scores() {
    assert_positive(SCORE_MAX);
    assert_positive(SCORE_MATCH_CONSECUTIVE);
    assert_positive(SCORE_MATCH_SLASH);
    assert_positive(SCORE_MATCH_WORD);
    assert_positive(SCORE_MATCH_CAPITAL);
    assert_positive(SCORE_MATCH_DOT);
  }

  #[test]
  fn negative_scores() {
    assert_negative(SCORE_MIN);
    assert_negative(SCORE_GAP_LEADING);
    assert_negative(SCORE_GAP_INNER);
    assert_negative(SCORE_GAP_TRAILING);
  }

  #[test]
  fn non_zero() {
    assert_ne!(0, CANDIDATE_MAX_BYTES);
    assert_ne!(0, CANDIDATE_MAX_CHARS);
  }
}
