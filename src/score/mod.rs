extern crate bit_vec;
extern crate ndarray;

pub mod config;

use std::cmp::Ordering;

use self::bit_vec::BitVec;
use self::ndarray::prelude::*;

use self::config::*;

pub type Score = f64;
type ScoreMatrix = Array2<Score>;

/// Result of querying the score against a candidate
#[derive(Debug)]
pub struct ScoreResult {
  pub candidate_index: usize,
  pub score: Score,
}

/// Result of querying the score and location against a candidate
#[derive(Debug)]
pub struct LocateResult {
  pub candidate_index: usize,
  pub score: Score,
  /// Binary mask showing where the charcaters of the query match the candidate
  pub match_mask: BitVec,
}

impl ScoreResult {
  pub fn new(candidate_index: usize) -> Self {
    Self::with_score(candidate_index, SCORE_MIN)
  }

  pub fn with_score(candidate_index: usize, score: Score) -> Self {
    Self {
      candidate_index,
      score,
    }
  }
}

impl PartialOrd for ScoreResult {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(
      self
        .score
        .partial_cmp(&other.score)
        .unwrap_or(Ordering::Less)
        .reverse(),
    )
  }
}

impl PartialEq for ScoreResult {
  fn eq(&self, other: &Self) -> bool {
    self.score == other.score
  }
}

impl LocateResult {
  pub fn new(candidate_index: usize, candidate_size: usize) -> Self {
    Self::with_score(candidate_index, candidate_size, SCORE_MIN)
  }

  pub fn with_score(candidate_index: usize, candidate_size: usize, score: Score) -> Self {
    Self {
      candidate_index,
      score: score,
      match_mask: BitVec::from_elem(candidate_size, false),
    }
  }
}

impl PartialOrd for LocateResult {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(
      self
        .score
        .partial_cmp(&other.score)
        .unwrap_or(Ordering::Less)
        .reverse(),
    )
  }
}

impl PartialEq for LocateResult {
  fn eq(&self, other: &Self) -> bool {
    self.score == other.score
  }
}

/// Returns `true` if and only if `candidate` is a match for `query`
///
/// A "match" must contain all of the letters of `query` in order, but not
/// necessarily continguously.
pub fn has_match(query: &str, candidate: &str) -> bool {
  let mut cand_iter = candidate.chars();
  // Note: `cand_iter` will be advanced during `all`, which is short-circuiting
  query
    .chars()
    .all(|c| cand_iter.any(|c2| c2.to_lowercase().eq(c.to_lowercase())))
}

/// Calculates a score for how well a `query` matches a `candidate`
///
/// Higher scores are better
pub fn score(query: &str, candidate: &str) -> ScoreResult {
  score_inner(query, candidate, 0)
}

pub(crate) fn score_inner(query: &str, candidate: &str, index: usize) -> ScoreResult {
  let (q_len, c_len) = match get_lengths(query, candidate) {
    LengthsOrScore::Score(s) => return ScoreResult::with_score(index, s),
    LengthsOrScore::Lengths(q, c) => (q, c),
  };

  let (best_score_overall, _) = score_internal(query, candidate, q_len, c_len);
  ScoreResult::with_score(index, best_score_overall[[q_len - 1, c_len - 1]])
}

/// Calculates a score for how well a `query` matches a `candidate` and gives
/// the locations of the `query` characters in the `candidate` too
///
/// Higher scores are better
pub fn locate(query: &str, candidate: &str) -> LocateResult {
  locate_inner(query, candidate, 0)
}

pub(crate) fn locate_inner(query: &str, candidate: &str, index: usize) -> LocateResult {
  let candidate_chars = candidate.chars().count();
  let (q_len, c_len) = match get_lengths(query, candidate) {
    LengthsOrScore::Score(s) => {
      let mut out = LocateResult::with_score(index, candidate_chars, s);
      if s == SCORE_MAX {
        // This was an exact match
        out.match_mask.set_all();
      }
      return out;
    }
    LengthsOrScore::Lengths(q, c) => (q, c),
  };

  let (best_score_overall, best_score_w_ending) = score_internal(query, candidate, q_len, c_len);
  let mut out = LocateResult::with_score(index, candidate_chars, best_score_overall[[q_len - 1, c_len - 1]]);

  let mut query_iter = query.chars();
  let mut cand_iter = candidate.chars();
  // Safe because we'll return at the beginning for zero or unit length
  let mut i = q_len;
  let mut j = c_len;
  while query_iter.next_back() != None {
    i = i.wrapping_sub(1);
    while cand_iter.next_back() != None {
      j = j.wrapping_sub(1);
      if best_score_w_ending[[i, j]] != SCORE_MIN
        && best_score_w_ending[[i, j]] == best_score_overall[[i, j]]
      {
        // There's a match here that was on an optimal path
        out.match_mask.set(j, true);
        break; // Go to the next query letter
      }
    }
  }

  out
}

enum LengthsOrScore {
  Lengths(usize, usize),
  Score(self::Score),
}

fn get_lengths(query: &str, candidate: &str) -> LengthsOrScore {
  if candidate.len() > CANDIDATE_MAX_BYTES || query.len() == 0 {
    // Candidate too long or query too short
    return LengthsOrScore::Score(SCORE_MIN);
  }

  let q_len = query.chars().count();
  let c_len = candidate.chars().count();

  if q_len == c_len {
    // This is only called when there _is_ a match (candidate contains all
    // chars of query in the right order, so equal lengths mean equal
    // strings
    return LengthsOrScore::Score(SCORE_MAX);
  }

  if c_len > CANDIDATE_MAX_CHARS {
    // Too many characters
    return LengthsOrScore::Score(SCORE_MIN);
  }

  LengthsOrScore::Lengths(q_len, c_len)
}

fn score_internal(
  query: &str,
  candidate: &str,
  q_len: usize,
  c_len: usize,
) -> (ScoreMatrix, ScoreMatrix) {
  let match_bonuses = candidate_match_bonuses(candidate);

  // Matrix of the best score for each position ending in a match
  let mut best_score_w_ending = ScoreMatrix::zeros((q_len, c_len));
  // Matrix for the best score for each position.
  let mut best_score_overall = ScoreMatrix::zeros((q_len, c_len));

  for (i, q_char) in query.chars().enumerate() {
    let mut prev_score = SCORE_MIN;
    let gap_score = if i == q_len - 1 {
      SCORE_GAP_TRAILING
    } else {
      SCORE_GAP_INNER
    };

    for (j, c_char) in candidate.chars().enumerate() {
      if q_char.to_lowercase().eq(c_char.to_lowercase()) {
        // Get the score bonus for matching this char
        let score = if i == 0 {
          // Beginning of the query, penalty for leading gap
          (j as f64 * SCORE_GAP_LEADING) + match_bonuses[j]
        } else if j != 0 {
          // Middle of both query and candidate
          // Either give it the match bonus, or use the consecutive
          // match (which wil always be higher, but doesn't stack
          // with match bonus)
          (best_score_overall[[i - 1, j - 1]] + match_bonuses[j])
            .max(best_score_w_ending[[i - 1, j - 1]] + SCORE_MATCH_CONSECUTIVE)
        } else {
          SCORE_MIN
        };

        prev_score = score.max(prev_score + gap_score);
        best_score_overall[[i, j]] = prev_score;
        best_score_w_ending[[i, j]] = score;
      } else {
        // Give the score penalty for the gap
        prev_score = prev_score + gap_score;
        best_score_overall[[i, j]] = prev_score;
        // We don't end in a match
        best_score_w_ending[[i, j]] = SCORE_MIN;
      }
    }
  }

  (best_score_overall, best_score_w_ending)
}

fn candidate_match_bonuses(candidate: &str) -> Vec<Score> {
  let mut prev_char = '/';
  candidate
    .chars()
    .map(|current| {
      let s = character_match_bonus(current, prev_char);
      prev_char = current;
      s
    })
    .collect()
}

fn character_match_bonus(current: char, previous: char) -> Score {
  if current.is_uppercase() && previous.is_lowercase() {
    SCORE_MATCH_CAPITAL
  } else {
    match previous {
      '/' => SCORE_MATCH_SLASH,
      '.' => SCORE_MATCH_DOT,
      _ if is_separator(previous) => SCORE_MATCH_WORD,
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn exact_match() {
    assert!(has_match("query", "query"));
    assert!(has_match(
      "156aufsdn926f9=sdk/~']",
      "156aufsdn926f9=sdk/~']"
    ));
    assert!(has_match(
      "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫",
      "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫"
    ));
  }

  #[test]
  fn paratial_match() {
    assert!(has_match("ca", "candidate"));
    assert!(has_match("cat", "candidate"));
    assert!(has_match("ndt", "candidate"));
    assert!(has_match("nate", "candidate"));
    assert!(has_match("56aufn92=sd/~']", "156aufsdn926f9=sdk/~']"));
    assert!(has_match(
      "😨Ɣ·®x¯ÍĞɅƁƹ♺à☆ǈ´ƙÑ♫",
      "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫"
    ));
  }

  #[test]
  fn case_match() {
    assert!(has_match("QUERY", "query"));
    assert!(has_match("query", "QUERY"));
    assert!(has_match("QuEry", "query"));
    assert!(has_match(
      "прописная буква",
      "ПРОПИСНАЯ БУКВА"
    ))
  }

  #[test]
  fn empty_match() {
    assert!(has_match("", ""));
    assert!(has_match("", "candidate"));
    assert!(has_match(
      "",
      "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫"
    ));
    assert!(has_match("", "прописная БУКВА"));
    assert!(has_match("", "a"));
    assert!(has_match("", "4561"));
  }

  #[test]
  fn bad_match() {
    assert!(!has_match("acb", "abc"));
    assert!(!has_match("a", ""));
    assert!(!has_match("abc", "def"));
    assert!(!has_match("😨Ɣ·®x¯ÍĞ.Ʌ", "5ù¨ȼ♕☩♘⚁^"));
    assert!(!has_match(
      "прописная БУКВА",
      "прописнаяБУКВА"
    ));
    assert!(!has_match(
      "БУКВА прописная",
      "прописная БУКВА"
    ));
  }

  #[test]
  fn score_pref_word_start() {
    assert!(score("amor", "app/models/order").score > score("amor", "app/models/zrder").score);
    assert!(score("amor", "app models-order").score > score("amor", "app models zrder").score);
    assert!(score("qart", "QuArTz").score > score("qart", "QuaRTz").score);
  }

  #[test]
  fn score_pref_consecutive_letters() {
    assert!(score("amo", "app/m/foo").score < score("amo", "app/models/foo").score);
  }

  #[test]
  fn score_pref_contiguous_vs_word() {
    assert!(score("gemfil", "Gemfile.lock").score < score("gemfil", "Gemfile").score);
  }

  #[test]
  fn score_pref_shorter() {
    assert!(score("abce", "abcdef").score > score("abce", "abc de").score);
    assert!(score("abc", "    a b c ").score > score("abc", " a  b  c ").score);
    assert!(score("abc", " a b c    ").score > score("abc", " a  b  c ").score);
    assert!(score("test", "tests").score > score("test", "testing").score);
  }

  #[test]
  fn score_prefer_start() {
    assert!(score("test", "testing").score > score("test", "/testing").score);
  }

  #[test]
  fn score_exact() {
    assert_eq!(SCORE_MAX, score("query", "query").score);
    assert_eq!(
      SCORE_MAX,
      score("156aufsdn926f9=sdk/~']", "156aufsdn926f9=sdk/~']").score
    );
    assert_eq!(
      SCORE_MAX,
      score(
        "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫",
        "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫"
      ).score
    );
  }

  #[test]
  fn score_empty() {
    assert_eq!(SCORE_MIN, score("", "").score);
    assert_eq!(SCORE_MIN, score("", "candidate").score);
    assert_eq!(
      SCORE_MIN,
      score(
        "",
        "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫"
      ).score
    );
    assert_eq!(SCORE_MIN, score("", "прописная БУКВА").score);
    assert_eq!(SCORE_MIN, score("", "a").score);
    assert_eq!(SCORE_MIN, score("", "4561").score);
  }

  #[test]
  fn score_gaps() {
    assert_eq!(SCORE_GAP_LEADING, score("a", "*a").score);
    assert_eq!(SCORE_GAP_LEADING * 2.0, score("a", "*ba").score);
    assert_eq!(
      SCORE_GAP_LEADING * 2.0 + SCORE_GAP_TRAILING,
      score("a", "**a*").score
    );
    assert_eq!(
      SCORE_GAP_LEADING * 2.0 + SCORE_GAP_TRAILING * 2.0,
      score("a", "**a**").score
    );
    assert_eq!(
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_CONSECUTIVE + SCORE_GAP_TRAILING * 2.0,
      score("aa", "**aa♺*").score
    );
    assert_eq!(
      SCORE_GAP_LEADING * 2.0 + SCORE_GAP_INNER + SCORE_MATCH_WORD + SCORE_GAP_TRAILING * 2.0,
      score("ab", "**a-b♺*").score
    );
    assert_eq!(
      SCORE_GAP_LEADING
        + SCORE_GAP_LEADING
        + SCORE_GAP_INNER
        + SCORE_GAP_TRAILING
        + SCORE_GAP_TRAILING,
      score("aa", "**a♺a**").score
    );
  }

  #[test]
  fn score_consecutive() {
    assert_eq!(
      SCORE_GAP_LEADING + SCORE_MATCH_CONSECUTIVE,
      score("aa", "*aa").score
    );
    assert_eq!(
      SCORE_GAP_LEADING + SCORE_MATCH_CONSECUTIVE * 2.0,
      score("aaa", "♫aaa").score
    );
    assert_eq!(
      SCORE_GAP_LEADING + SCORE_GAP_INNER + SCORE_MATCH_CONSECUTIVE,
      score("aaa", "*a*aa").score
    );
  }

  #[test]
  fn score_slash() {
    assert_eq!(
      SCORE_GAP_LEADING + SCORE_MATCH_SLASH,
      score("a", "/a").score
    );
    assert_eq!(
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_SLASH,
      score("a", "*/a").score
    );
    assert_eq!(
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_SLASH + SCORE_MATCH_CONSECUTIVE,
      score("aa", "a/aa").score
    );
  }

  #[test]
  fn score_capital() {
    assert_eq!(
      SCORE_GAP_LEADING + SCORE_MATCH_CAPITAL,
      score("a", "bA").score
    );
    assert_eq!(
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_CAPITAL,
      score("a", "baA").score
    );
    assert_eq!(
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_CAPITAL + SCORE_MATCH_CONSECUTIVE,
      score("aa", "😞aAa").score
    );
  }

  #[test]
  fn score_dot() {
    assert_eq!(SCORE_GAP_LEADING + SCORE_MATCH_DOT, score("a", ".a").score);
    assert_eq!(
      SCORE_GAP_LEADING * 3.0 + SCORE_MATCH_DOT,
      score("a", "*a.a").score
    );
    assert_eq!(
      SCORE_GAP_LEADING + SCORE_GAP_INNER + SCORE_MATCH_DOT,
      score("a", "♫a.a").score
    );
  }

  fn assert_locate_score(query: &str, candidate: &str, score: Score) {
    let result = locate(query, candidate);

    assert_eq!(score, result.score);
  }

  #[test]
  fn locate_exact() {
    assert_locate_score("query", "query", SCORE_MAX);
    assert_locate_score("156aufsdn926f9=sdk/~']",
      "156aufsdn926f9=sdk/~']",
      SCORE_MAX,
    );
    assert_locate_score(
        "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫",
      "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫",
      SCORE_MAX,
    );
  }

  #[test]
  fn locate_empty() {
    assert_locate_score("", "", SCORE_MIN);
    assert_locate_score("", "candidate", SCORE_MIN);
    assert_locate_score(
        "",
        "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫, ",
      SCORE_MIN,
    );
    assert_locate_score("", "прописная БУКВА", SCORE_MIN);
    assert_locate_score("", "a", SCORE_MIN);
    assert_locate_score("", "4561", SCORE_MIN);
  }

  #[test]
  fn locate_gaps() {
    assert_locate_score("a", "*a", SCORE_GAP_LEADING);
    assert_locate_score("a", "*ba", SCORE_GAP_LEADING * 2.0);
    assert_locate_score("a", "**a*",
      SCORE_GAP_LEADING * 2.0 + SCORE_GAP_TRAILING,
    );
    assert_locate_score("a", "**a**",
      SCORE_GAP_LEADING * 2.0 + SCORE_GAP_TRAILING * 2.0,
    );
    assert_locate_score("aa", "**aa♺*",
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_CONSECUTIVE + SCORE_GAP_TRAILING * 2.0,
    );
    assert_locate_score("ab", "**a-b♺*",
      SCORE_GAP_LEADING * 2.0 + SCORE_GAP_INNER + SCORE_MATCH_WORD + SCORE_GAP_TRAILING * 2.0,
    );
    assert_locate_score("aa", "**a♺a**",
      SCORE_GAP_LEADING
        + SCORE_GAP_LEADING
        + SCORE_GAP_INNER
        + SCORE_GAP_TRAILING
        + SCORE_GAP_TRAILING,
    );
  }

  #[test]
  fn locate_consecutive() {
    assert_locate_score("aa", "*aa",
      SCORE_GAP_LEADING + SCORE_MATCH_CONSECUTIVE,
    );
    assert_locate_score("aaa", "♫aaa",
      SCORE_GAP_LEADING + SCORE_MATCH_CONSECUTIVE * 2.0,
    );
    assert_locate_score("aaa", "*a*aa",
      SCORE_GAP_LEADING + SCORE_GAP_INNER + SCORE_MATCH_CONSECUTIVE,
    );
  }

  #[test]
  fn locate_slash() {
    assert_locate_score("a", "/a",
      SCORE_GAP_LEADING + SCORE_MATCH_SLASH,
    );
    assert_locate_score("a", "*/a",
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_SLASH,
    );
    assert_locate_score("aa", "a/aa",
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_SLASH + SCORE_MATCH_CONSECUTIVE,
    );
  }

  #[test]
  fn locate_capital() {
    assert_locate_score("a", "bA",
      SCORE_GAP_LEADING + SCORE_MATCH_CAPITAL,
    );
    assert_locate_score("a", "baA",
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_CAPITAL,
    );
    assert_locate_score("aa", "😞aAa",
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_CAPITAL + SCORE_MATCH_CONSECUTIVE,
    );
  }

  #[test]
  fn locate_dot() {
    assert_locate_score("a", ".a", SCORE_GAP_LEADING + SCORE_MATCH_DOT);
    assert_locate_score("a", "*a.a",
      SCORE_GAP_LEADING * 3.0 + SCORE_MATCH_DOT,
    );
    assert_locate_score("a", "♫a.a",
      SCORE_GAP_LEADING + SCORE_GAP_INNER + SCORE_MATCH_DOT,
    );
  }

}
