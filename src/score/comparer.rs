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
  match_bonuses: Vec<f64>,
}

impl<'q, 'c> Comparer<'q, 'c> {
  /// Create a new comparer, unless we can short-circuit because of some
  /// trivial case (e.g., no match at all)
  pub fn new(query: &'q str, candidate: &'c str) -> Result<Self, Error> {
    // TODO: checking for a match can be done at the same time as calculating
    // the lengths
    if !is_match(query, candidate) {
      return Err(Error::ShortCircuit(SCORE_MIN));
    }

    if candidate.len() > CANDIDATE_MAX_BYTES || query.len() == 0 {
      // Candidate too long or query too short
      return Err(Error::ShortCircuit(SCORE_MIN));
    }

    let q_len = query.chars().count();
    let c_len = candidate.chars().count();

    if q_len == c_len {
      // We already know there _is_ a match (candidate contains chars of query
      // in the right order), so equal lengths mean equal strings with the
      // current algorithm
      return Err(Error::ShortCircuit(SCORE_MAX));
    }

    if c_len > CANDIDATE_MAX_CHARS {
      return Err(Error::ShortCircuit(SCORE_MIN));
    }

    let mut bonuses = Vec::new();
    Self::candidate_match_bonuses(&mut bonuses, candidate, c_len);

    Ok(Self {
      query: query,
      candidate: candidate,
      q_len: q_len,
      c_len: c_len,
      best_score_overall: ScoreMatrix::zeros((q_len, c_len)),
      best_score_ending: ScoreMatrix::zeros((q_len, c_len)),
      match_bonuses: bonuses,
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
    for (i, q_char) in self.query.chars().enumerate() {
      let mut prev_score = SCORE_MIN;
      let gap_score = if i == self.q_len - 1 {
        SCORE_GAP_TRAILING
      } else {
        SCORE_GAP_INNER
      };

      for (j, c_char) in self.candidate.chars().enumerate() {
        if q_char.to_lowercase().eq(c_char.to_lowercase()) {
          // Get the score bonus for matching this char
          let score = if i == 0 {
            // Beginning of the query, penalty for leading gap
            (j as Score * SCORE_GAP_LEADING) + self.match_bonuses[j]
          } else if j != 0 {
            // Middle of both query and candidate
            // Either give it the match bonus, or use the consecutive
            // match (which wil always be higher, but doesn't stack
            // with match bonus)
            (self.best_score_overall[[i - 1, j - 1]] + self.match_bonuses[j])
              .max(self.best_score_ending[[i - 1, j - 1]] + SCORE_MATCH_CONSECUTIVE)
          } else {
            SCORE_MIN
          };

          prev_score = score.max(prev_score + gap_score);
          self.best_score_overall[[i, j]] = prev_score;
          self.best_score_ending[[i, j]] = score;
        } else {
          // Give the score penalty for the gap
          prev_score = prev_score + gap_score;
          self.best_score_overall[[i, j]] = prev_score;
          // We don't end in a match
          self.best_score_ending[[i, j]] = SCORE_MIN;
        }
      }
    }
  }

  fn locate_internal(&mut self, mask: &mut MatchMask) {
    let mut query_iter = self.query.chars();
    let mut cand_iter = self.candidate.chars();
    let mut i = self.q_len;
    let mut j = self.c_len;
    while query_iter.next_back() != None {
      i = i.wrapping_sub(1);
      while cand_iter.next_back() != None {
        j = j.wrapping_sub(1);
        if self.best_score_ending[[i, j]] != SCORE_MIN
          && self.best_score_ending[[i, j]] == self.best_score_overall[[i, j]]
        {
          // There's a match here that was on an optimal path
          mask.set(j, true);
          break; // Go to the next query letter
        } else {
          mask.set(j, false);
        }
      }
    }
  }

  fn candidate_match_bonuses(out: &mut Vec<Score>, candidate: &str, candidate_length: usize) {
    let mut prev_char = '/';
    out.resize(candidate_length, 0.0);
    let mut i = 0;
    candidate.chars().for_each(|current| {
      out[i] = Self::character_match_bonus(current, prev_char);
      prev_char = current;
      i += 1;
    });
  }

  fn character_match_bonus(current: char, previous: char) -> Score {
    if current.is_uppercase() && previous.is_lowercase() {
      SCORE_MATCH_CAPITAL
    } else {
      match previous {
        '/' => SCORE_MATCH_SLASH,
        '.' => SCORE_MATCH_DOT,
        _ if Self::is_separator(previous) => SCORE_MATCH_WORD,
        _ => 0.0,
      }
    }
  }

  fn is_separator(character: char) -> bool {
    match character {
      ' ' => true,
      '-' => true,
      '_' => true,
      _ => false,
    }
  }
}

//==============================================================================

use score::config::*;
use score::{is_match, MatchMask, Score};

type ScoreMatrix = ndarray::Array2<Score>;

//==============================================================================

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn character_bonuses() {
    assert_eq!(0.0, Comparer::character_match_bonus('a', 'b'));
    assert_eq!(0.0, Comparer::character_match_bonus('0', '#'));
    assert_eq!(0.0, Comparer::character_match_bonus('¬', '9'));
    assert_eq!(0.0, Comparer::character_match_bonus('@', 'b'));
    assert_eq!(0.0, Comparer::character_match_bonus('&', ','));
    assert_eq!(0.0, Comparer::character_match_bonus('😨', '♫'));
    assert_eq!(0.0, Comparer::character_match_bonus('♺', 'ƹ'));
    assert_eq!(0.0, Comparer::character_match_bonus('x', '¯'));
    assert_eq!(0.0, Comparer::character_match_bonus('Ɣ', '®'));
    assert_eq!(0.0, Comparer::character_match_bonus(']', '·'));
    assert_eq!(0.0, Comparer::character_match_bonus('A', 'B'));
    assert_eq!(0.0, Comparer::character_match_bonus('a', 'B'));
    assert_eq!(0.0, Comparer::character_match_bonus('Б', 'Б'));
    assert_eq!(0.0, Comparer::character_match_bonus('и', 'Б'));

    assert_eq!(
      SCORE_MATCH_CAPITAL,
      Comparer::character_match_bonus('G', 'r')
    );
    assert_eq!(
      SCORE_MATCH_CAPITAL,
      Comparer::character_match_bonus('Б', 'и')
    );

    assert_eq!(SCORE_MATCH_SLASH, Comparer::character_match_bonus('a', '/'));
    assert_eq!(SCORE_MATCH_SLASH, Comparer::character_match_bonus('0', '/'));
    assert_eq!(SCORE_MATCH_SLASH, Comparer::character_match_bonus('¬', '/'));
    assert_eq!(SCORE_MATCH_SLASH, Comparer::character_match_bonus('@', '/'));
    assert_eq!(SCORE_MATCH_SLASH, Comparer::character_match_bonus('&', '/'));
    assert_eq!(SCORE_MATCH_SLASH, Comparer::character_match_bonus('😨', '/'));
    assert_eq!(SCORE_MATCH_SLASH, Comparer::character_match_bonus('♺', '/'));
    assert_eq!(SCORE_MATCH_SLASH, Comparer::character_match_bonus('x', '/'));
    assert_eq!(SCORE_MATCH_SLASH, Comparer::character_match_bonus('Ɣ', '/'));
    assert_eq!(SCORE_MATCH_SLASH, Comparer::character_match_bonus(']', '/'));
    assert_eq!(SCORE_MATCH_SLASH, Comparer::character_match_bonus('A', '/'));
    assert_eq!(SCORE_MATCH_SLASH, Comparer::character_match_bonus('Б', '/'));
    assert_eq!(SCORE_MATCH_SLASH, Comparer::character_match_bonus('и', '/'));

    assert_eq!(SCORE_MATCH_DOT, Comparer::character_match_bonus('a', '.'));
    assert_eq!(SCORE_MATCH_DOT, Comparer::character_match_bonus('0', '.'));
    assert_eq!(SCORE_MATCH_DOT, Comparer::character_match_bonus('¬', '.'));
    assert_eq!(SCORE_MATCH_DOT, Comparer::character_match_bonus('@', '.'));
    assert_eq!(SCORE_MATCH_DOT, Comparer::character_match_bonus('&', '.'));
    assert_eq!(SCORE_MATCH_DOT, Comparer::character_match_bonus('😨', '.'));
    assert_eq!(SCORE_MATCH_DOT, Comparer::character_match_bonus('♺', '.'));
    assert_eq!(SCORE_MATCH_DOT, Comparer::character_match_bonus('x', '.'));
    assert_eq!(SCORE_MATCH_DOT, Comparer::character_match_bonus('Ɣ', '.'));
    assert_eq!(SCORE_MATCH_DOT, Comparer::character_match_bonus(']', '.'));
    assert_eq!(SCORE_MATCH_DOT, Comparer::character_match_bonus('A', '.'));
    assert_eq!(SCORE_MATCH_DOT, Comparer::character_match_bonus('Б', '.'));
    assert_eq!(SCORE_MATCH_DOT, Comparer::character_match_bonus('и', '.'));

    assert_eq!(SCORE_MATCH_WORD, Comparer::character_match_bonus('a', ' '));
    assert_eq!(SCORE_MATCH_WORD, Comparer::character_match_bonus('0', '-'));
    assert_eq!(SCORE_MATCH_WORD, Comparer::character_match_bonus('¬', '_'));
    assert_eq!(SCORE_MATCH_WORD, Comparer::character_match_bonus('@', ' '));
    assert_eq!(SCORE_MATCH_WORD, Comparer::character_match_bonus('&', '-'));
    assert_eq!(SCORE_MATCH_WORD, Comparer::character_match_bonus('😨', '_'));
    assert_eq!(SCORE_MATCH_WORD, Comparer::character_match_bonus('♺', ' '));
    assert_eq!(SCORE_MATCH_WORD, Comparer::character_match_bonus('x', '-'));
    assert_eq!(SCORE_MATCH_WORD, Comparer::character_match_bonus('Ɣ', '_'));
    assert_eq!(SCORE_MATCH_WORD, Comparer::character_match_bonus(']', ' '));
    assert_eq!(SCORE_MATCH_WORD, Comparer::character_match_bonus('A', '-'));
    assert_eq!(SCORE_MATCH_WORD, Comparer::character_match_bonus('Б', '_'));
    assert_eq!(SCORE_MATCH_WORD, Comparer::character_match_bonus('и', ' '));
  }

}
