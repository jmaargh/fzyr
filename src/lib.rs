pub mod score;
mod search;

pub use score::{config, is_match, score, PositionalCandidate, Score};
// FIXME
// pub use search::{search_locate, search_score, LocateResults, ScoreResults};
