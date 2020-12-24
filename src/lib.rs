mod score;
mod search;

pub use score::{config, has_match, locate, score, LocateResult, Score, ScoreResult};
pub use search::{search_serial, locate_serial, LocateResults, ScoreResults};

#[cfg(feature = "parallel")]
pub use search::{search_locate, search_score};
