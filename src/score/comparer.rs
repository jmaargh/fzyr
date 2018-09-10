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
  best_score: Score,
  trace_matrix: TraceMatrix,
  match_bonuses: Vec<f64>,
}

impl<'q, 'c> Comparer<'q, 'c> {
  /// Create a new comparer, unless we can short-circuit because of some
  /// trivial case (e.g., no match at all)
  pub fn new(query: &'q str, candidate: &'c str) -> Result<Self, Error> {
    if candidate.len() > CANDIDATE_MAX_BYTES || query.len() == 0 {
      // Candidate too long or query too short
      return Err(Error::ShortCircuit(SCORE_MIN));
    }

    let (q_len, c_len) = Self::match_count(query, candidate)?;

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

    // N.B. We need an extra row and column to account for trailing gaps and
    // work out the optimal path
    Ok(Self {
      query: query,
      candidate: candidate,
      q_len: q_len,
      c_len: c_len,
      best_score: SCORE_MIN,
      trace_matrix: TraceMatrix::from_elem((q_len + 1, c_len + 1), TraceEdges::new()),
      match_bonuses: bonuses,
    })
  }

  /// Calculate the score
  pub fn score(&mut self) -> Score {
    self.score_internal();
    self.best_score
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
    self.best_score
  }

  fn score_internal(&mut self) {
    // We can skip candidate characters too close to the beginning to be
    // possible matches. We can also skip candidate characters too close to the
    // end to leave enough room for the rest of the matches. The result being
    // that we only ever need to look at this many c_chars per q_char
    let c_chars_per_iter = self.c_len - self.q_len + 1;

    let mut q_iter = self.query.chars();
    for i in 0..=self.q_len {
      let q_char = q_iter.next().unwrap_or('\0'); // Assumes `\0` will not match
                                                  // anything

      let gap_score = if i == 0 {
        SCORE_GAP_LEADING
      } else if i == self.q_len {
        SCORE_GAP_TRAILING
      } else {
        SCORE_GAP_INNER
      };

      let max_c_bound = if i == self.q_len {
        self.c_len + 1
      } else {
        i + c_chars_per_iter
      };

      let mut c_iter = self.candidate.chars().skip(i);
      for j in i..max_c_bound {
        // N.B. we need the `_or` value here to not match the `_or` value from
        // before
        let c_char = c_iter.next().unwrap_or(' ');

        // Scores for previous being a match, and not, respectively
        let (mut yes_score, mut no_score) = if i == 0 {
          // First row
          (SCORE_MIN, SCORE_GAP_LEADING * j as Score)
        } else {
          // neither `i` nor `j` can be zero here
          (
            self.trace_matrix[[i - 1, j - 1]].yes.score,
            self.trace_matrix[[i, j - 1]].no.score,
          )
        };

        // Get the best score link for (i,j) not being a match
        if yes_score > no_score {
          self.trace_matrix[[i, j]].no.prev_was_match = true;
          self.trace_matrix[[i, j]].no.score = yes_score;
        } else {
          self.trace_matrix[[i, j]].no.prev_was_match = false;
          self.trace_matrix[[i, j]].no.score = no_score;
        }
        if j != self.c_len {
          self.trace_matrix[[i, j]].no.score += gap_score;
        }

        if q_char.to_lowercase().eq(c_char.to_lowercase()) {
          // Get the best score link for (i, j) being a match
          yes_score += SCORE_MATCH_CONSECUTIVE;
          no_score += self.match_bonuses[j];

          if yes_score >= no_score {
            self.trace_matrix[[i, j]].yes.prev_was_match = true;
            self.trace_matrix[[i, j]].yes.score = yes_score;
          } else {
            self.trace_matrix[[i, j]].yes.prev_was_match = false;
            self.trace_matrix[[i, j]].yes.score = no_score;
          }
        }
      }
    }

    self.best_score = self.trace_matrix[[self.q_len, self.c_len]].no.score;
  }

  fn locate_internal(&mut self, mask: &mut MatchMask) {
    mask.clear();

    let mut i = self.q_len;
    let mut j = self.c_len;
    let mut was_match = false;
    while i != 0 {
      was_match = if was_match {
        // We'll never be out of bounds here, because the [q_len, ..] row
        // never matches
        mask.set(j, true);
        i -= 1;
        self.trace_matrix[[i, j]].yes.prev_was_match
      } else {
        self.trace_matrix[[i, j]].no.prev_was_match
      };
      j -= 1;
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

  fn match_count(query: &str, candidate: &str) -> Result<(usize, usize), Error> {
    // In a single pass, check whether `candidate` is a match for `query` and
    // record their lengths

    let mut q_len = 0;
    let mut c_len = 0;

    let mut q_iter = query.chars();
    let mut c_iter = candidate.chars();

    while let Some(cq) = q_iter.next() {
      q_len += 1;
      loop {
        c_len += 1;
        match c_iter.next() {
          Some(cc) => if cc.to_lowercase().eq(cq.to_lowercase()) {
            break;
          },
          None => return Err(Error::ShortCircuit(SCORE_MIN)),
        }
      }
    }

    c_len += c_iter.count();
    Ok((q_len, c_len))
  }
}

//==============================================================================

use score::config::*;
use score::{MatchMask, Score};

type TraceMatrix = ndarray::Array2<TraceEdges>;

#[derive(Copy, Clone)]
struct TraceEdges {
  yes: TraceEdge,
  no: TraceEdge,
}

impl TraceEdges {
  fn new() -> Self {
    Self {
      yes: TraceEdge::new(),
      no: TraceEdge::new(),
    }
  }
}

#[derive(Copy, Clone)]
struct TraceEdge {
  score: Score,
  prev_was_match: bool,
}

impl TraceEdge {
  fn new() -> Self {
    Self {
      score: SCORE_MIN,
      prev_was_match: false,
    }
  }
}

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
    assert_eq!(
      SCORE_MATCH_SLASH,
      Comparer::character_match_bonus('¬', '/')
    );
    assert_eq!(SCORE_MATCH_SLASH, Comparer::character_match_bonus('@', '/'));
    assert_eq!(SCORE_MATCH_SLASH, Comparer::character_match_bonus('&', '/'));
    assert_eq!(
      SCORE_MATCH_SLASH,
      Comparer::character_match_bonus('😨', '/')
    );
    assert_eq!(
      SCORE_MATCH_SLASH,
      Comparer::character_match_bonus('♺', '/')
    );
    assert_eq!(SCORE_MATCH_SLASH, Comparer::character_match_bonus('x', '/'));
    assert_eq!(
      SCORE_MATCH_SLASH,
      Comparer::character_match_bonus('Ɣ', '/')
    );
    assert_eq!(SCORE_MATCH_SLASH, Comparer::character_match_bonus(']', '/'));
    assert_eq!(SCORE_MATCH_SLASH, Comparer::character_match_bonus('A', '/'));
    assert_eq!(
      SCORE_MATCH_SLASH,
      Comparer::character_match_bonus('Б', '/')
    );
    assert_eq!(
      SCORE_MATCH_SLASH,
      Comparer::character_match_bonus('и', '/')
    );

    assert_eq!(SCORE_MATCH_DOT, Comparer::character_match_bonus('a', '.'));
    assert_eq!(SCORE_MATCH_DOT, Comparer::character_match_bonus('0', '.'));
    assert_eq!(SCORE_MATCH_DOT, Comparer::character_match_bonus('¬', '.'));
    assert_eq!(SCORE_MATCH_DOT, Comparer::character_match_bonus('@', '.'));
    assert_eq!(SCORE_MATCH_DOT, Comparer::character_match_bonus('&', '.'));
    assert_eq!(
      SCORE_MATCH_DOT,
      Comparer::character_match_bonus('😨', '.')
    );
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
    assert_eq!(
      SCORE_MATCH_WORD,
      Comparer::character_match_bonus('😨', '_')
    );
    assert_eq!(
      SCORE_MATCH_WORD,
      Comparer::character_match_bonus('♺', ' ')
    );
    assert_eq!(SCORE_MATCH_WORD, Comparer::character_match_bonus('x', '-'));
    assert_eq!(SCORE_MATCH_WORD, Comparer::character_match_bonus('Ɣ', '_'));
    assert_eq!(SCORE_MATCH_WORD, Comparer::character_match_bonus(']', ' '));
    assert_eq!(SCORE_MATCH_WORD, Comparer::character_match_bonus('A', '-'));
    assert_eq!(SCORE_MATCH_WORD, Comparer::character_match_bonus('Б', '_'));
    assert_eq!(SCORE_MATCH_WORD, Comparer::character_match_bonus('и', ' '));
  }

}
