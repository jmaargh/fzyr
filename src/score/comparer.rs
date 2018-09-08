extern crate ndarray;

pub enum Error {
  // Variant returned when we can short-circuit the logic of the comparer and
  // return a string straight away with either `SCORE_MIN` or `SCORE_MAX`
  ShortCircuit(Score),
}

pub struct Comparer<'q, 'c> {
  query: &'q str,
  candidate: &'c str,
  q_len: usize,
  c_len: usize,
  best_score_overall: ScoreMatrix,
  best_score_ending: ScoreMatrix,
}

impl<'q, 'c> Comparer<'q, 'c> {
  /// Create a new comparer, unless we can short-circuit because of some
  /// trivial case (e.g., no match at all)
  pub fn new(query: &'q str, candidate: &'c str) -> Result<Self, Error> {
    if !is_match(query, candidate) {
      return Err(Error::ShortCircuit(SCORE_MIN));
    }

    if candidate.len() > CANDIDATE_MAX_BYTES || query.len() == 0 {
      // Candidate too long or query too short
      return Err(Error::ShortCircuit(SCORE_MIN));
    }

    let q_len = query.chars().count();
    let c_len = query.chars().count();

    if q_len == c_len {
      // We already know there _is_ a match (candidate contains chars of query
      // in the right order), so equal lengths mean equal strings with the
      // current algorithm
      return Err(Error::ShortCircuit(SCORE_MAX));
    }

    if c_len > CANDIDATE_MAX_CHARS {
      return Err(Error::ShortCircuit(SCORE_MIN));
    }

    Ok(Self {
      query: query,
      candidate: candidate,
      q_len: q_len,
      c_len: c_len,
      best_score_overall: ScoreMatrix::zeros((q_len, c_len)),
      best_score_ending: ScoreMatrix::zeros((q_len, c_len)),
    })
  }

  /// Calculate the score
  pub fn score(&mut self) -> Score {
    self.score_internal();
    self.best_score_overall[[self.q_len - 1, self.c_len - 1]]
  }

  /// Calculate the score and mark the given match according to where the
  /// matching characters are
  ///
  /// If the given mask is the wrong size, clear it and return `SCORE_MIN`
  pub fn locate(&mut self, mask: &mut MatchMask) -> Score {
    if mask.len() != self.c_len {
      mask.clear();
      return SCORE_MIN;
    }

    self.score_internal();
    self.locate_internal(mask);
    self.best_score_overall[[self.q_len - 1, self.c_len - 1]]
  }

  fn score_internal(&mut self) {
    // TODO: implement
  }

  fn locate_internal(&mut self, mask: &mut MatchMask) {
    // TODO: implement
  }
}

//==============================================================================

use self::ndarray::prelude::*;

use score::config::*;
use score::{is_match, MatchMask, Score};

type ScoreMatrix = Array2<Score>;
