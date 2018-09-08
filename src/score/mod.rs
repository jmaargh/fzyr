extern crate bit_vec;

pub mod config;

pub use self::config::{SCORE_MAX, SCORE_MIN};

/// How well a candidate matches the query, higher is better
pub type Score = f64;

/// A reusable candidate which can locate a query within itself
pub struct PositionalCandidate<'c> {
  candidate: &'c str,
  length: usize,
  match_mask: MatchMask,
  score: Score,
  has_searched: bool,
}

impl<'c> PositionalCandidate<'c> {
  /// Create a new candidate with no matches
  pub fn new(candidate: &'c str) -> Self {
    let length = Self::calculate_len(candidate);
    Self {
      candidate: candidate,
      length: length,
      match_mask: MatchMask::from_elem(length, false),
      score: SCORE_MIN,
      has_searched: false,
    }
  }

  /// Calculate how well the candidate matches the given query and locate the
  /// characters of the query within the candidate
  ///
  /// Non-matches return `SCORE_MIN`, exact matches return `SCORE_MAX`. Queries
  /// or candidates that are too long return `SCORE_MIN`.
  pub fn locate(&mut self, query: &str) -> Score {
    if self.has_searched {
      return self.score;
    }

    let mut comparer = match Comparer::new(query, self.candidate) {
      Ok(comp) => comp,
      Err(comparer::Error::ShortCircuit(score)) => return self.locate_short_circuit(score),
    };

    let out = comparer.locate(&mut self.match_mask);
    self.has_searched = true;
    out
  }

  pub fn reset(&mut self) {
    self.has_searched = false;
    self.match_mask.clear();
    self.score = SCORE_MIN;
  }

  pub fn reset_candidate(&mut self, new_candidate: &'c str) {
    self.candidate = new_candidate;
    self.length = Self::calculate_len(self.candidate);
    self.resize_mask();
    self.reset();
  }

  /// Get the candidate
  #[inline]
  pub fn candidate(&self) -> &'c str {
    &self.candidate
  }

  /// Get a bit mask for which characters in the candidate match the query
  ///
  /// Returns all zero if the current candidate hasn't been searched against
  #[inline]
  pub fn match_mask<'a>(&'a self) -> &'a MatchMask {
    &self.match_mask
  }

  /// Get the previously calculated score
  ///
  /// Returns `SCORE_MIN` if the current candidate hasn't been searched against
  #[inline]
  pub fn score(&self) -> Score {
    self.score
  }

  /// Returns true if and only if this candidate has been searched agaisnt
  #[inline]
  pub fn has_searched(&self) -> bool {
    self.has_searched
  }

  fn locate_short_circuit(&mut self, score: Score) -> Score {
    // Set the mask properly when short-circuiting with SCORE_MAX or SCORE_MIN
    self.has_searched = true;
    if score == SCORE_MAX {
      self.match_mask.clear();
      SCORE_MAX
    } else {
      self.match_mask.set_all();
      SCORE_MIN
    }
  }

  fn resize_mask(&mut self) {
    let old_length = self.match_mask.len();
    match self.length.overflowing_sub(old_length) {
      (extension, false) => self.match_mask.grow(extension, false),
      _ => self.match_mask.truncate(old_length - self.length),
    }
  }

  fn calculate_len(s: &str) -> usize {
    s.chars().count()
  }
}

/// Return `true` if and only if `candidate` is a match for `query`
///
/// A "match" contains all of the characters of `query` in the correct order,
/// ignoring case and contiguity.
pub fn is_match(query: &str, candidate: &str) -> bool {
  let mut cand_iter = candidate.chars();
  // Note: `cand_iter` will be advanced during `all`, which is short-circuiting
  query
    .chars()
    .all(|cq| cand_iter.any(|cc| cc.to_lowercase().eq(cq.to_lowercase())))
}

/// Calculate how well the given candidate matches the given query
///
/// Non-matches return `SCORE_MIN`, exact matches return `SCORE_MAX`. Queries
/// or candidates that are too long return `SCORE_MIN`.
pub fn score(query: &str, candidate: &str) -> Score {
  let mut comparer = match Comparer::new(query, candidate) {
    Ok(comp) => comp,
    Err(comparer::Error::ShortCircuit(score)) => return score,
  };

  comparer.score()
}

//==============================================================================

mod comparer;

use self::bit_vec::BitVec;

use self::comparer::Comparer;

type MatchMask = BitVec;
