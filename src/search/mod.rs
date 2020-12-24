use std::cmp::Ordering;
use std::iter::ExactSizeIterator;

use score::{has_match, locate_inner, score_inner, LocateResult, ScoreResult};

#[cfg(feature = "parallel")]
mod parallel;

#[cfg(feature = "parallel")]
pub use self::parallel::{search_score, search_locate};

/// Collection of scores and the candidates they apply to
pub type ScoreResults = Vec<ScoreResult>;
/// Collection of scores, locations, and the candidates they apply to
pub type LocateResults = Vec<LocateResult>;

pub fn search_serial(
  query: &str,
  candidates: impl Iterator<Item = impl AsRef<str>> + ExactSizeIterator,
) -> ScoreResults {
  search_worker(candidates, query, 0, score_inner)
}

pub fn locate_serial(
  query: &str,
  candidates: impl Iterator<Item = impl AsRef<str>> + ExactSizeIterator,
) -> LocateResults {
  search_worker(candidates, query, 0, locate_inner)
}

// Search among candidates against a query in a single thread
fn search_worker<T>(
  candidates: impl IntoIterator<Item = impl AsRef<str>> + ExactSizeIterator,
  query: &str,
  offset_index: usize,
  search_fn: fn(&str, &str, usize) -> T
) -> Vec<T>
where
  T: PartialOrd,
{
  let mut out = Vec::with_capacity(candidates.len());
  for (index, candidate) in candidates.into_iter().enumerate() {
    let candidate = candidate.as_ref();
    if has_match(&query, candidate) {
      out.push(search_fn(&query, candidate, offset_index + index));
    }
  }
  out.sort_unstable_by(|result1, result2| result1.partial_cmp(result2).unwrap_or(Ordering::Less));

  out
}

