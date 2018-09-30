pub struct SearchResult<'c> {
  pub score: Score,
  pub candidate: &'c str,
}

pub type SearchResults<'c> = Vec<SearchResult<'c>>;

pub struct Searcher<'c> {
  results: SearchResults<'c>,
}

impl<'c> Searcher<'c> {
  pub fn new(candidates: &[&'c str]) -> Self {
    let mut results = Vec::with_capacity(candidates.len());
    for candidate in candidates {
      results.push(SearchResult {
        score: SCORE_MIN,
        candidate: *candidate,
      });
    }
    Self { results }
  }

  pub fn search(&mut self, query: &str) -> &SearchResults<'c> {
    search_worker(query, &mut self.results, &search_actual);
    // Sort highest score first
    self.results.sort_by(|a, b| {
      a.score
        .partial_cmp(&b.score)
        .unwrap_or(Ordering::Less)
        .reverse()
    });
    &self.results
  }
}

pub type LocateResults<'c> = Vec<Box<PositionalCandidate<'c>>>;

pub struct Locator<'c> {
  results: LocateResults<'c>,
}

impl<'c> Locator<'c> {
  pub fn new(candidates: &[&'c str]) -> Self {
    let mut results = Vec::with_capacity(candidates.len());
    for candidate in candidates {
      results.push(Box::new(PositionalCandidate::<'c>::new(candidate)));
    }
    Self { results }
  }

  pub fn locate(&mut self, query: &str) -> &LocateResults<'c> {
    search_worker(query, &mut self.results, &locate_actual);
    // Search by highest score first
    self.results.sort_by(|a, b| {
      a.score()
        .partial_cmp(&b.score())
        .unwrap_or(Ordering::Less)
        .reverse()
    });
    &self.results
  }
}

//==============================================================================

use std::cmp::Ordering;

use score::{score, PositionalCandidate, Score, SCORE_MIN};

fn search_worker<F, R>(query: &str, collection: &mut [R], search_fn: &'static F)
where
  F: Fn(&str, &mut R) -> (),
{
  for item in collection {
    search_fn(query, item);
  }
}

fn search_actual(query: &str, result: &mut SearchResult) {
  result.score = score(query, result.candidate);
}

fn locate_actual(query: &str, result: &mut Box<PositionalCandidate>) {
  result.locate(query);
}

#[cfg(test)]
mod tests {
  use super::*;
  use score::config::{SCORE_GAP_LEADING, SCORE_MATCH_CONSECUTIVE};

  #[test]
  fn search_empty() {
    let mut sr = Searcher::new(&[]);

    {
      let rs = sr.search("");
      assert_eq!(0, rs.len());
    }
    {
      let rs = sr.search("test");
      assert_eq!(0, rs.len());
    }
  }

  #[test]
  fn search_short() {
    let mut sr = Searcher::new(&["tags"]);
    let rs = sr.search("");
    assert_eq!(1, rs.len());
    assert_eq!("tags", rs[0].candidate);

    let mut sr = Searcher::new(&["ñîƹ♺à"]);
    let rs = sr.search("♺");
    assert_eq!(1, rs.len());
    assert_eq!("ñîƹ♺à", rs[0].candidate);

    let mut sr = Searcher::new(&["tags", "test"]);
    {
      let rs = sr.search("");
      assert_eq!(2, rs.len());
      for r in rs {
        assert_eq!(r.score, SCORE_MIN);
      }
    }
    {
      let rs = sr.search("te");
      assert_eq!("test", rs[0].candidate);
      assert_eq!("tags", rs[1].candidate);
      assert_eq!(SCORE_MIN, rs[1].score);
    }
    {
      let rs = sr.search("foobar");
      assert_eq!(2, rs.len());
      for r in rs {
        assert_eq!(r.score, SCORE_MIN);
      }
    }
    {
      let rs = sr.search("ts");
      assert_eq!(2, rs.len());
      for r in rs {
        assert_ne!(r.score, SCORE_MIN);
      }
    }
  }

  #[test]
  fn search_med() {
    let cs = &[
      "one",
      "two",
      "three",
      "four",
      "five",
      "six",
      "seven",
      "eight",
      "nine",
      "ten",
      "eleven",
      "twelve",
      "thirteen",
      "fourteen",
      "fifteen",
      "sixteen",
      "seventeen",
      "eighteen",
      "nineteen",
      "twenty",
    ];

    let mut sr = Searcher::new(cs);

    {
      let rs = sr.search("");
      assert_eq!(cs.len(), rs.len());
    }
    {
      let rs = sr.search("teen");
      assert_eq!(cs.len(), rs.len());
      let mut iter = rs.iter();
      let r = iter.next().unwrap();
      assert_eq!(r.candidate, "fifteen");
      assert_eq!(
        r.score,
        3.0 * SCORE_GAP_LEADING + 3.0 * SCORE_MATCH_CONSECUTIVE
      );
      for r in iter.by_ref().take(6) {
        assert_eq!(
          "neet",
          r.candidate.chars().rev().take(4).collect::<String>()
        );
        assert!(r.score > 0.0);
      }
      for r in iter {
        assert_eq!(r.score, SCORE_MIN);
      }
    }
    {
      let rs = sr.search("tee");
      let mut iter = rs.iter();
      for r in iter.by_ref().take(7) {
        assert_ne!(r.score, SCORE_MIN);
        assert_eq!(
          "neet",
          r.candidate.chars().rev().take(4).collect::<String>()
        );
      }
      let r = iter.next().unwrap();
      assert_ne!(r.score, SCORE_MIN);
      assert_eq!(r.candidate, "three");
      let r = iter.next().unwrap();
      assert_ne!(r.score, SCORE_MIN);
      assert_eq!(r.candidate, "twelve");
      for r in iter {
        assert_eq!(r.score, SCORE_MIN);
      }
    }
    {
      let rs = sr.search("six");
      assert_eq!("six", rs[0].candidate);
    }
  }

  #[test]
  fn search_large() {
    let n = 100_000;
    let mut candidates = Vec::with_capacity(n);
    for i in 0..n {
      candidates.push(format!("{}", i));
    }

    let mut sr = Searcher::new(&(candidates.iter().map(|s| &s[..]).collect::<Vec<&str>>()));

    let rs = sr.search("12");

    // This has been precalculated
    // e.g. via `$ seq 0 99999 | grep '.*1.*2.*' | wc -l`
    assert_eq!("12", rs[0].candidate);
    let mut iter = rs.iter();
    for r in iter.by_ref().take(8146) {
      assert_ne!(r.score, SCORE_MIN)
    }
    for r in iter {
      assert_eq!(r.score, SCORE_MIN);
    }
  }

  // TODO: test locate
}
