use super::*;

/// Search among a collection of candidates using the given query, returning
/// an ordered collection of results (highest score first).
///
/// # Example
///
/// ```rust
/// # use fzyr::search_score;
/// let items = vec!["this", "is", "kind", "of", "magic"];
/// let res = search_score("mgc", &items, 1);
/// assert_eq!("magic", items[res[0].candidate_index]);
/// ```
pub fn search_score(
  query: &str,
  candidates: &[&str],
  parallelism: usize,
) -> ScoreResults {
  search_internal(query, candidates, parallelism, score_inner).collect()
}

/// Search among a collection of candidates using the given query, returning
/// an ordered collection of results (highest score first) with the locations
/// of the query in each candidate.
///
/// # Example
///
/// ```rust
/// # use fzyr::search_locate;
/// let items = vec!["this", "is", "kind", "of", "magic"];
/// let res = search_locate("mgc", &items, 1);
/// assert_eq!("magic", items[res[0].candidate_index]);
/// ```
pub fn search_locate(
  query: &str,
  candidates: &[&str],
  parallelism: usize,
) -> LocateResults {
  search_internal(query, candidates, parallelism, locate_inner).collect()
}

fn search_internal<T>(
  query: &str,
  candidates: &[&str],
  parallelism: usize,
  search_fn: fn(&str, &str, usize) -> T,
) -> Box<dyn Iterator<Item = T>>
where
  T: PartialOrd + Sized + Send + 'static,
{
  let parallelism = calculate_parallelism(candidates.len(), parallelism, query.is_empty());
  let mut candidates = candidates;
  let (sender, receiver) = crossbeam::channel::bounded::<Vec<T>>(parallelism);

  if parallelism < 2 {
    Box::new(search_worker(candidates.iter(), query, 0, search_fn).into_iter())
  } else {
    let _ = crossbeam::scope(|scope| {
      let mut remaining_candidates = candidates.len();
      let per_thread_count = ceil_div(remaining_candidates, parallelism);
      let mut thread_offset = 0;

      // Create "parallelism" threads
      while remaining_candidates > 0 {
        // Search in this thread's share
        let split = if remaining_candidates >= per_thread_count {
          remaining_candidates -= per_thread_count;
          per_thread_count
        } else {
          remaining_candidates = 0;
          remaining_candidates
        };
        let split = candidates.split_at(split);
        let splitted_len = split.0.len();
        let sender = sender.clone();
        scope.spawn(move |_| {
          let _ = sender.send(search_worker(split.0.iter(), query, thread_offset, search_fn));
        });
        thread_offset += splitted_len;

        // Remove that share from the candidate slice
        candidates = split.1;
      }

      drop(sender);
    });

    Box::new(itertools::kmerge(receiver))
  }
}

fn calculate_parallelism(
  candidate_count: usize,
  configured_parallelism: usize,
  empty_query: bool,
) -> usize {
  if empty_query {
    // No need to do much for no query
    return 1;
  }

  // Use a ramp up to avoid unecessarily starting threads with few candidates
  let ramped_parallelism = match candidate_count {
    n if n < 17 => ceil_div(n, 4),
    n if n > 32 => ceil_div(n, 8),
    _ => 4,
  };

  configured_parallelism
    .min(ramped_parallelism)
    .min(candidate_count)
    .max(1)
}

/// Integer ceiling division
fn ceil_div(a: usize, b: usize) -> usize {
  (a + b - 1) / b
}

#[cfg(test)]
mod tests {
  use std::usize;

  use super::*;

  #[test]
  fn parallelism_ramp() {
    assert_eq!(1, calculate_parallelism(0, 0, false));
    assert_eq!(1, calculate_parallelism(1, 0, false));
    assert_eq!(1, calculate_parallelism(0, 1, false));
    assert_eq!(1, calculate_parallelism(1, 1, false));

    assert_eq!(1, calculate_parallelism(2, usize::MAX, false));
    assert_eq!(1, calculate_parallelism(3, 4, false));
    assert_eq!(1, calculate_parallelism(4, 2, false));

    for n in 5..9 {
      assert_eq!(2, calculate_parallelism(n, usize::MAX, false));
      assert_eq!(1, calculate_parallelism(n, usize::MAX, true));
    }

    for n in 9..13 {
      assert_eq!(3, calculate_parallelism(n, usize::MAX, false));
      assert_eq!(1, calculate_parallelism(n, usize::MAX, true));
    }

    for n in 13..33 {
      assert_eq!(4, calculate_parallelism(n, usize::MAX, false));
      assert_eq!(1, calculate_parallelism(n, usize::MAX, true));
    }

    for n in 1..10_000 {
      assert!(calculate_parallelism(n, 12, false) <= 12);
      assert_eq!(1, calculate_parallelism(n, 12, true));
    }
  }

  fn search_empty_with_parallelism(parallelism: usize) {
    let rs = search_score("", &[], parallelism);
    assert_eq!(0, rs.len());

    let rs = search_score("test", &[], parallelism);
    assert_eq!(0, rs.len());
  }

  fn search_with_parallelism(parallelism: usize) {
    search_empty_with_parallelism(parallelism);

    let rs = search_score("", &["tags"], parallelism);
    assert_eq!(1, rs.len());
    assert_eq!(0, rs[0].candidate_index);

    let rs = search_score("♺", &["ñîƹ♺à"], parallelism);
    assert_eq!(1, rs.len());
    assert_eq!(0, rs[0].candidate_index);

    let cs = &["tags", "test"];

    let rs = search_score("", cs, parallelism);
    assert_eq!(2, rs.len());

    let rs = search_score("te", cs, parallelism);
    assert_eq!(1, rs.len());
    assert_eq!(1, rs[0].candidate_index);

    let rs = search_score("foobar", cs, parallelism);
    assert_eq!(0, rs.len());

    let rs = search_score("ts", cs, parallelism);
    assert_eq!(2, rs.len());
    assert_eq!(
      vec![1, 0],
      rs.iter().map(|r| r.candidate_index).collect::<Vec<_>>()
    );
  }

  fn search_med_parallelism(parallelism: usize) {
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

    let rs = search_score("", cs, parallelism);
    assert_eq!(cs.len(), rs.len());

    let rs = search_score("teen", cs, parallelism);
    assert_eq!(7, rs.len());
    for r in rs {
      assert_eq!(
        "neet",
        cs[r.candidate_index].chars().rev().take(4).collect::<String>()
      );
    }

    let rs = search_score("tee", cs, parallelism);
    assert_eq!(9, rs.len());
    assert_eq!(
      "neet",
      cs[rs[0].candidate_index].chars().rev().take(4).collect::<String>()
    );

    let rs = search_score("six", cs, parallelism);
    assert_eq!("six", cs[rs[0].candidate_index]);
  }

  fn search_large_parallelism(parallelism: usize) {
    let n = 100_000;
    let mut candidates = Vec::with_capacity(n);
    for i in 0..n {
      candidates.push(format!("{}", i));
    }

    let rs = search_score(
      "12",
      &(candidates.iter().map(|s| &s[..]).collect::<Vec<&str>>()),
      parallelism,
    );

    // This has been precalculated
    // e.g. via `$ seq 0 99999 | grep '.*1.*2.*' | wc -l`
    assert_eq!(8146, rs.len());
    assert_eq!("12", candidates[rs[0].candidate_index]);
  }

  // TODO: test locate

  #[test]
  fn search_single() {
    search_with_parallelism(0);
    search_with_parallelism(1);
    search_large_parallelism(1);
  }

  #[test]
  fn search_double() {
    search_with_parallelism(2);
    search_large_parallelism(2);
  }

  #[test]
  fn search_quad() {
    search_med_parallelism(4);
    search_large_parallelism(4);
  }

  #[test]
  fn search_quin() {
    search_med_parallelism(4);
    search_large_parallelism(5);
  }

  #[test]
  fn search_large() {
    search_med_parallelism(4);
    search_large_parallelism(16);
  }
}
