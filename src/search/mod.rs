use std::cmp::Ordering;

use score::{has_match, locate_inner, score_inner, LocateResult, ScoreResult};

#[cfg(feature = "parallel")]
mod parallel;

#[cfg(feature = "parallel")]
pub use self::parallel::{search_score, search_locate};

/// Collection of scores and the candidates they apply to
pub type ScoreResults = Vec<ScoreResult>;
/// Collection of scores, locations, and the candidates they apply to
pub type LocateResults = Vec<LocateResult>;

/// Search serially among a collection of candidates using the given query, returning
/// an ordered collection of results (highest score first).
///
/// # Example
///
/// ```rust
/// # use fzyr::search_serial;
/// let items = vec!["this", "is", "kind", "of", "magic"];
/// let res = search_serial("mgc", items.iter());
/// assert_eq!("magic", items[res[0].candidate_index]);
/// ```
pub fn search_serial(
  query: &str,
  candidates: impl IntoIterator<Item = impl AsRef<str>>,
) -> ScoreResults {
  search_worker(candidates, query, 0, score_inner)
}

/// Search serially among a collection of candidates using the given query, returning
/// an ordered collection of results (highest score first) with the locations
/// of the query in each candidate.
///
/// # Example
///
/// ```rust
/// # use fzyr::locate_serial;
/// let items = vec!["this", "is", "kind", "of", "magic"];
/// let res = locate_serial("mgc", items.iter());
/// assert_eq!("magic", items[res[0].candidate_index]);
/// ```
pub fn locate_serial(
  query: &str,
  candidates: impl IntoIterator<Item = impl AsRef<str>>,
) -> LocateResults {
  search_worker(candidates, query, 0, locate_inner)
}

// Search among candidates against a query in a single thread
fn search_worker<T>(
  candidates: impl IntoIterator<Item = impl AsRef<str>>,
  query: &str,
  offset_index: usize,
  search_fn: fn(&str, &str, usize) -> T
) -> Vec<T>
where
  T: PartialOrd,
{
  let candidates = candidates.into_iter();
  let (low, high) = candidates.size_hint();
  let mut out = Vec::with_capacity(high.unwrap_or(low));
  for (index, candidate) in candidates.enumerate() {
    let candidate = candidate.as_ref();
    if has_match(&query, candidate) {
      out.push(search_fn(&query, candidate, offset_index + index));
    }
  }
  out.sort_unstable_by(|result1, result2| result1.partial_cmp(result2).unwrap_or(Ordering::Less));

  out
}

