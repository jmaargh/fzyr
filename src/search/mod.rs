// extern crate crossbeam;
// extern crate itertools;

// use std::cmp::Ordering;
// use std::usize;

// use self::crossbeam::channel;
// use self::crossbeam::scope as thread_scope;
// use self::itertools::kmerge;

// use score::{has_match, locate, score, LocateResult, ScoreResult};

// /// Collection of scores and the candidates they apply to
// pub type ScoreResults<'a> = Vec<ScoreResult<'a>>;
// /// Collection of scores, locations, and the candidates they apply to
// pub type LocateResults<'a> = Vec<LocateResult<'a>>;

// /// Search among a collection of candidates using the given query, returning
// /// an ordered collection of results (highest score first)
// pub fn search_score<'src>(
//   query: &str,
//   candidates: &[&'src str],
//   parallelism: usize,
// ) -> ScoreResults<'src> {
//   search_internal(query, candidates, parallelism, &score).collect()
// }

// /// Search among a collection of candidates using the given query, returning
// /// an ordered collection of results (highest score first) with the locations
// /// of the query in each candidate
// pub fn search_locate<'src>(
//   query: &str,
//   candidates: &[&'src str],
//   parallelism: usize,
// ) -> LocateResults<'src> {
//   search_internal(query, candidates, parallelism, &locate).collect()
// }

// fn search_internal<'a, F, T>(
//   query: &str,
//   candidates: &[&'a str],
//   parallelism: usize,
//   search_fn: &'static F,
// ) -> Box<Iterator<Item = T> + 'a>
// where
//   T: PartialOrd + Sized + Send + 'a,
//   F: Fn(&str, &'a str) -> T + Sync,
// {
//   let parallelism = calculate_parallelism(candidates.len(), parallelism, query.is_empty());
//   let mut candidates = candidates;
//   let (sender, receiver) = channel::bounded::<Vec<T>>(parallelism);

//   if parallelism < 2 {
//     Box::new(search_worker(candidates, query, search_fn).into_iter())
//   } else {
//     thread_scope(|scope| {
//       let mut remaining_candidates = candidates.len();
//       let per_thread_count = ceil_div(remaining_candidates, parallelism);

//       // Create "parallelism" threads
//       while remaining_candidates > 0 {
//         // Search in this thread's share
//         let split = candidates.split_at(if remaining_candidates >= per_thread_count {
//           remaining_candidates -= per_thread_count;
//           per_thread_count
//         } else {
//           remaining_candidates = 0;
//           remaining_candidates
//         });
//         let sender = sender.clone();
//         scope.spawn(move || {
//           sender.send(search_worker(split.0, query, search_fn));
//         });

//         // Remove that share from the candidate slice
//         candidates = split.1;
//       }

//       drop(sender);
//     });

//     Box::new(kmerge(receiver))
//   }
// }

// // Search among candidates against a query in a single thread
// fn search_worker<'a, 'b, F, T>(candidates: &'b [&'a str], query: &'b str, search_fn: F) -> Vec<T>
// where
//   T: PartialOrd,
//   F: Fn(&str, &'a str) -> T,
// {
//   let mut out = Vec::with_capacity(candidates.len());
//   for candidate in candidates {
//     if has_match(&query, candidate) {
//       out.push(search_fn(&query, candidate));
//     }
//   }
//   out.sort_unstable_by(|result1, result2| result1.partial_cmp(result2).unwrap_or(Ordering::Less));

//   out
// }

// fn calculate_parallelism(
//   candidate_count: usize,
//   configured_parallelism: usize,
//   empty_query: bool,
// ) -> usize {
//   if empty_query {
//     // No need to do much for no query
//     return 1;
//   }

//   // Use a ramp up to avoid unecessarily starting threads with few candidates
//   let ramped_parallelism = match candidate_count {
//     n if n < 17 => ceil_div(n, 4),
//     n if n > 32 => ceil_div(n, 8),
//     _ => 4,
//   };

//   configured_parallelism
//     .min(ramped_parallelism)
//     .min(candidate_count)
//     .max(1)
// }

// /// Integer ceiling division
// fn ceil_div(a: usize, b: usize) -> usize {
//   (a + b - 1) / b
// }

// #[cfg(test)]
// mod tests {
//   use super::*;

//   #[test]
//   fn parallelism_ramp() {
//     assert_eq!(1, calculate_parallelism(0, 0, false));
//     assert_eq!(1, calculate_parallelism(1, 0, false));
//     assert_eq!(1, calculate_parallelism(0, 1, false));
//     assert_eq!(1, calculate_parallelism(1, 1, false));

//     assert_eq!(1, calculate_parallelism(2, usize::MAX, false));
//     assert_eq!(1, calculate_parallelism(3, 4, false));
//     assert_eq!(1, calculate_parallelism(4, 2, false));

//     for n in 5..9 {
//       assert_eq!(2, calculate_parallelism(n, usize::MAX, false));
//       assert_eq!(1, calculate_parallelism(n, usize::MAX, true));
//     }

//     for n in 9..13 {
//       assert_eq!(3, calculate_parallelism(n, usize::MAX, false));
//       assert_eq!(1, calculate_parallelism(n, usize::MAX, true));
//     }

//     for n in 13..33 {
//       assert_eq!(4, calculate_parallelism(n, usize::MAX, false));
//       assert_eq!(1, calculate_parallelism(n, usize::MAX, true));
//     }

//     for n in 1..10_000 {
//       assert!(calculate_parallelism(n, 12, false) <= 12);
//       assert_eq!(1, calculate_parallelism(n, 12, true));
//     }
//   }

//   fn search_empty_with_parallelism(parallelism: usize) {
//     let rs = search_score("", &[], parallelism);
//     assert_eq!(0, rs.len());

//     let rs = search_score("test", &[], parallelism);
//     assert_eq!(0, rs.len());
//   }

//   fn search_with_parallelism(parallelism: usize) {
//     search_empty_with_parallelism(parallelism);

//     let rs = search_score("", &["tags"], parallelism);
//     assert_eq!(1, rs.len());
//     assert_eq!("tags", rs[0].candidate);

//     let rs = search_score("♺", &["ñîƹ♺à"], parallelism);
//     assert_eq!(1, rs.len());
//     assert_eq!("ñîƹ♺à", rs[0].candidate);

//     let cs = &["tags", "test"];

//     let rs = search_score("", cs, parallelism);
//     assert_eq!(2, rs.len());

//     let rs = search_score("te", cs, parallelism);
//     assert_eq!(1, rs.len());
//     assert_eq!("test", rs[0].candidate);

//     let rs = search_score("foobar", cs, parallelism);
//     assert_eq!(0, rs.len());

//     let rs = search_score("ts", cs, parallelism);
//     assert_eq!(2, rs.len());
//     assert_eq!(
//       vec!["test", "tags"],
//       rs.iter()
//         .map(|r| r.candidate)
//         .collect::<Vec<&'static str>>()
//     );
//   }

//   fn search_med_parallelism(parallelism: usize) {
//     let cs = &[
//       "one",
//       "two",
//       "three",
//       "four",
//       "five",
//       "six",
//       "seven",
//       "eight",
//       "nine",
//       "ten",
//       "eleven",
//       "twelve",
//       "thirteen",
//       "fourteen",
//       "fifteen",
//       "sixteen",
//       "seventeen",
//       "eighteen",
//       "nineteen",
//       "twenty",
//     ];

//     let rs = search_score("", cs, parallelism);
//     assert_eq!(cs.len(), rs.len());

//     let rs = search_score("teen", cs, parallelism);
//     assert_eq!(7, rs.len());
//     for r in rs {
//       assert_eq!(
//         "neet",
//         r.candidate.chars().rev().take(4).collect::<String>()
//       );
//     }

//     let rs = search_score("tee", cs, parallelism);
//     assert_eq!(9, rs.len());
//     assert_eq!(
//       "neet",
//       rs[0].candidate.chars().rev().take(4).collect::<String>()
//     );

//     let rs = search_score("six", cs, parallelism);
//     assert_eq!("six", rs[0].candidate);
//   }

//   fn search_large_parallelism(parallelism: usize) {
//     let n = 100_000;
//     let mut candidates = Vec::with_capacity(n);
//     for i in 0..n {
//       candidates.push(format!("{}", i));
//     }

//     let rs = search_score(
//       "12",
//       &(candidates.iter().map(|s| &s[..]).collect::<Vec<&str>>()),
//       parallelism,
//     );

//     // This has been precalculated
//     // e.g. via `$ seq 0 99999 | grep '.*1.*2.*' | wc -l`
//     assert_eq!(8146, rs.len());
//     assert_eq!("12", rs[0].candidate);
//   }

//   // TODO: test locate

//   #[test]
//   fn search_single() {
//     search_with_parallelism(0);
//     search_with_parallelism(1);
//     search_large_parallelism(1);
//   }

//   #[test]
//   fn search_double() {
//     search_with_parallelism(2);
//     search_large_parallelism(2);
//   }

//   #[test]
//   fn search_quad() {
//     search_med_parallelism(4);
//     search_large_parallelism(4);
//   }

//   #[test]
//   fn search_quin() {
//     search_med_parallelism(4);
//     search_large_parallelism(5);
//   }

//   #[test]
//   fn search_large() {
//     search_med_parallelism(4);
//     search_large_parallelism(16);
//   }
// }
