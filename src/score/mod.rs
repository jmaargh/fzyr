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

    self.score = comparer.locate(&mut self.match_mask);
    self.has_searched = true;
    self.score
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
      self.match_mask.set_all();
      self.score = SCORE_MAX;
    } else {
      self.match_mask.clear();
      self.score = SCORE_MIN;
    }
    self.score
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

#[cfg(test)]
mod tests {
  use super::config::{
    SCORE_GAP_INNER, SCORE_GAP_LEADING, SCORE_GAP_TRAILING, SCORE_MATCH_CAPITAL,
    SCORE_MATCH_CONSECUTIVE, SCORE_MATCH_DOT, SCORE_MATCH_SLASH, SCORE_MATCH_WORD,
  };
  use super::*;

  #[test]
  fn exact_match() {
    assert!(is_match("query", "query"));
    assert!(is_match("156aufsdn926f9=sdk/~']", "156aufsdn926f9=sdk/~']"));
    assert!(is_match(
      "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫",
      "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫"
    ));
  }

  #[test]
  fn paratial_match() {
    assert!(is_match("ca", "candidate"));
    assert!(is_match("cat", "candidate"));
    assert!(is_match("ndt", "candidate"));
    assert!(is_match("nate", "candidate"));
    assert!(is_match("56aufn92=sd/~']", "156aufsdn926f9=sdk/~']"));
    assert!(is_match(
      "😨Ɣ·®x¯ÍĞɅƁƹ♺à☆ǈ´ƙÑ♫",
      "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫"
    ));
  }

  #[test]
  fn case_match() {
    assert!(is_match("QUERY", "query"));
    assert!(is_match("query", "QUERY"));
    assert!(is_match("QuEry", "query"));
    assert!(is_match(
      "прописная буква",
      "ПРОПИСНАЯ БУКВА"
    ))
  }

  #[test]
  fn empty_match() {
    assert!(is_match("", ""));
    assert!(is_match("", "candidate"));
    assert!(is_match(
      "",
      "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫"
    ));
    assert!(is_match("", "прописная БУКВА"));
    assert!(is_match("", "a"));
    assert!(is_match("", "4561"));
  }

  #[test]
  fn bad_match() {
    assert!(!is_match("acb", "abc"));
    assert!(!is_match("a", ""));
    assert!(!is_match("abc", "def"));
    assert!(!is_match("😨Ɣ·®x¯ÍĞ.Ʌ", "5ù¨ȼ♕☩♘⚁^"));
    assert!(!is_match(
      "прописная БУКВА",
      "прописнаяБУКВА"
    ));
    assert!(!is_match(
      "БУКВА прописная",
      "прописная БУКВА"
    ));
  }

  #[test]
  fn score_pref_word_start() {
    assert!(score("amor", "app/marrows/older") > score("amor", "app/marrows/zlder"));
    assert!(score("amor", "app marrows-older") > score("amor", "app marrows zlder"));
    assert!(score("qart", "QuArTz") > score("qart", "QuaRTz"));
  }

  #[test]
  fn score_pref_consecutive_letters() {
    assert!(score("amo", "app/m/foo") < score("amo", "app/models/foo"));
  }

  #[test]
  fn score_pref_contiguous_vs_word() {
    assert!(score("gemfil", "Gemfile.lock") < score("gemfil", "Gemfile"));
  }

  #[test]
  fn score_pref_shorter() {
    assert!(score("abce", "abcdef") > score("abce", "abc de"));
    assert!(score("abc", "    a b c ") > score("abc", " a  b  c "));
    assert!(score("abc", " a b c    ") > score("abc", " a  b  c "));
    assert!(score("test", "tests") > score("test", "testing"));
  }

  #[test]
  fn score_prefer_start() {
    assert!(score("test", "testing") > score("test", "/testing"));
  }

  #[test]
  fn score_exact() {
    assert_eq!(SCORE_MAX, score("query", "query"));
    assert_eq!(
      SCORE_MAX,
      score("156aufsdn926f9=sdk/~']", "156aufsdn926f9=sdk/~']")
    );
    assert_eq!(
      SCORE_MAX,
      score(
        "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫",
        "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫"
      )
    );
  }

  #[test]
  fn score_empty() {
    assert_eq!(SCORE_MIN, score("", ""));
    assert_eq!(SCORE_MIN, score("", "candidate"));
    assert_eq!(
      SCORE_MIN,
      score(
        "",
        "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫"
      )
    );
    assert_eq!(SCORE_MIN, score("", "прописная БУКВА"));
    assert_eq!(SCORE_MIN, score("", "a"));
    assert_eq!(SCORE_MIN, score("", "4561"));
  }

  #[test]
  fn score_gaps() {
    assert_eq!(SCORE_GAP_LEADING, score("a", "*a"));
    assert_eq!(SCORE_GAP_LEADING * 2.0, score("a", "*ba"));
    assert_eq!(
      SCORE_GAP_LEADING * 2.0 + SCORE_GAP_TRAILING,
      score("a", "**a*")
    );
    assert_eq!(
      SCORE_GAP_LEADING * 2.0 + SCORE_GAP_TRAILING * 2.0,
      score("a", "**a**")
    );
    assert_eq!(
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_CONSECUTIVE + SCORE_GAP_TRAILING * 2.0,
      score("aa", "**aa♺*")
    );
    assert_eq!(
      SCORE_GAP_LEADING * 2.0 + SCORE_GAP_INNER + SCORE_MATCH_WORD + SCORE_GAP_TRAILING * 2.0,
      score("ab", "**a-b♺*")
    );
    assert_eq!(
      SCORE_GAP_LEADING
        + SCORE_GAP_LEADING
        + SCORE_GAP_INNER
        + SCORE_GAP_TRAILING
        + SCORE_GAP_TRAILING,
      score("aa", "**a♺a**")
    );
  }

  #[test]
  fn score_consecutive() {
    assert_eq!(
      SCORE_GAP_LEADING + SCORE_MATCH_CONSECUTIVE,
      score("aa", "*aa")
    );
    assert_eq!(
      SCORE_GAP_LEADING + SCORE_MATCH_CONSECUTIVE * 2.0,
      score("aaa", "♫aaa")
    );
    assert_eq!(
      SCORE_GAP_LEADING + SCORE_GAP_INNER + SCORE_MATCH_CONSECUTIVE,
      score("aaa", "*a*aa")
    );
  }

  #[test]
  fn score_slash() {
    assert_eq!(SCORE_GAP_LEADING + SCORE_MATCH_SLASH, score("a", "/a"));
    assert_eq!(
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_SLASH,
      score("a", "*/a")
    );
    assert_eq!(
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_SLASH + SCORE_MATCH_CONSECUTIVE,
      score("aa", "a/aa")
    );
  }

  #[test]
  fn score_capital() {
    assert_eq!(SCORE_GAP_LEADING + SCORE_MATCH_CAPITAL, score("a", "bA"));
    assert_eq!(
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_CAPITAL,
      score("a", "baA")
    );
    assert_eq!(
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_CAPITAL + SCORE_MATCH_CONSECUTIVE,
      score("aa", "😞aAa")
    );
  }

  #[test]
  fn score_dot() {
    assert_eq!(SCORE_GAP_LEADING + SCORE_MATCH_DOT, score("a", ".a"));
    assert_eq!(
      SCORE_GAP_LEADING * 3.0 + SCORE_MATCH_DOT,
      score("a", "*a.a")
    );
    assert_eq!(
      SCORE_GAP_LEADING + SCORE_GAP_INNER + SCORE_MATCH_DOT,
      score("a", "♫a.a")
    );
  }

  fn assert_eq_locate(result: PositionalCandidate, query: &str, score: Score) {
    assert_eq!(score, result.score);
    let mut found_query = String::new();
    for (i, ch) in result.candidate.chars().enumerate() {
      if result.match_mask[i] {
        found_query.push(ch);
      }
    }
    assert_eq!(query.to_lowercase(), found_query.to_lowercase());
  }

  fn locate<'c>(query: &str, candidate: &'c str) -> PositionalCandidate<'c> {
    let mut out = PositionalCandidate::new(candidate);
    out.locate(query);
    out
  }

  #[test]
  fn locate_exact() {
    assert_eq_locate(locate("query", "query"), "query", SCORE_MAX);
    assert_eq_locate(
      locate("156aufsdn926f9=sdk/~']", "156aufsdn926f9=sdk/~']"),
      "156aufsdn926f9=sdk/~']",
      SCORE_MAX,
    );
    assert_eq_locate(
      locate(
        "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫",
        "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫",
      ),
      "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫",
      SCORE_MAX,
    );
  }

  #[test]
  fn locate_empty() {
    assert_eq_locate(locate("", ""), "", SCORE_MIN);
    assert_eq_locate(locate("", "candidate"), "", SCORE_MIN);
    assert_eq_locate(
      locate(
        "",
        "😨Ɣ·®x¯ÍĞ.ɅƁñîƹ♺àwÑ☆ǈ😞´ƙºÑ♫, ",
      ),
      "",
      SCORE_MIN,
    );
    assert_eq_locate(locate("", "прописная БУКВА"), "", SCORE_MIN);
    assert_eq_locate(locate("", "a"), "", SCORE_MIN);
    assert_eq_locate(locate("", "4561"), "", SCORE_MIN);
  }

  #[test]
  fn locate_gaps() {
    assert_eq_locate(locate("a", "*a"), "a", SCORE_GAP_LEADING);
    assert_eq_locate(locate("a", "*ba"), "a", SCORE_GAP_LEADING * 2.0);
    assert_eq_locate(
      locate("a", "**a*"),
      "a",
      SCORE_GAP_LEADING * 2.0 + SCORE_GAP_TRAILING,
    );
    assert_eq_locate(
      locate("a", "**a**"),
      "a",
      SCORE_GAP_LEADING * 2.0 + SCORE_GAP_TRAILING * 2.0,
    );
    assert_eq_locate(
      locate("aa", "**aa♺*"),
      "aa",
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_CONSECUTIVE + SCORE_GAP_TRAILING * 2.0,
    );
    assert_eq_locate(
      locate("ab", "**a-b♺*"),
      "ab",
      SCORE_GAP_LEADING * 2.0 + SCORE_GAP_INNER + SCORE_MATCH_WORD + SCORE_GAP_TRAILING * 2.0,
    );
    assert_eq_locate(
      locate("aa", "**a♺a**"),
      "aa",
      SCORE_GAP_LEADING
        + SCORE_GAP_LEADING
        + SCORE_GAP_INNER
        + SCORE_GAP_TRAILING
        + SCORE_GAP_TRAILING,
    );
  }

  #[test]
  fn locate_consecutive() {
    assert_eq_locate(
      locate("aa", "*aa"),
      "aa",
      SCORE_GAP_LEADING + SCORE_MATCH_CONSECUTIVE,
    );
    assert_eq_locate(
      locate("aaa", "♫aaa"),
      "aaa",
      SCORE_GAP_LEADING + SCORE_MATCH_CONSECUTIVE * 2.0,
    );
    assert_eq_locate(
      locate("aaa", "*a*aa"),
      "aaa",
      SCORE_GAP_LEADING + SCORE_GAP_INNER + SCORE_MATCH_CONSECUTIVE,
    );
  }

  #[test]
  fn locate_slash() {
    assert_eq_locate(
      locate("a", "/a"),
      "a",
      SCORE_GAP_LEADING + SCORE_MATCH_SLASH,
    );
    assert_eq_locate(
      locate("a", "*/a"),
      "a",
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_SLASH,
    );
    assert_eq_locate(
      locate("aa", "a/aa"),
      "aa",
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_SLASH + SCORE_MATCH_CONSECUTIVE,
    );
  }

  #[test]
  fn locate_capital() {
    assert_eq_locate(
      locate("a", "bA"),
      "a",
      SCORE_GAP_LEADING + SCORE_MATCH_CAPITAL,
    );
    assert_eq_locate(
      locate("a", "baA"),
      "a",
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_CAPITAL,
    );
    assert_eq_locate(
      locate("aa", "😞aAa"),
      "aa",
      SCORE_GAP_LEADING * 2.0 + SCORE_MATCH_CAPITAL + SCORE_MATCH_CONSECUTIVE,
    );
  }

  #[test]
  fn locate_dot() {
    assert_eq_locate(locate("a", ".a"), "a", SCORE_GAP_LEADING + SCORE_MATCH_DOT);
    assert_eq_locate(
      locate("a", "*a.a"),
      "a",
      SCORE_GAP_LEADING * 3.0 + SCORE_MATCH_DOT,
    );
    assert_eq_locate(
      locate("a", "♫a.a"),
      "a",
      SCORE_GAP_LEADING + SCORE_GAP_INNER + SCORE_MATCH_DOT,
    );
  }

}
