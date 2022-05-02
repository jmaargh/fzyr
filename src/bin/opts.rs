extern crate clap;

use self::clap::{Command, Arg};

pub const NAME: &'static str = env!("CARGO_PKG_NAME");
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const WEBSITE: &'static str = env!("CARGO_PKG_HOMEPAGE");
pub const DESCRIPTION: &'static str = env!("CARGO_PKG_DESCRIPTION");

pub const DEFLT_STRING_BUFFER_LEN: usize = 128;

#[derive(Debug)]
pub struct Options {
  pub query: String,
  pub lines: usize,
  pub show_scores: bool,
  pub parallelism: usize,
  pub prompt: String,
  pub benchmark: usize,
}

impl Default for Options {
  fn default() -> Self {
    Self {
      query: String::new(),
      lines: 10,
      show_scores: false,
      parallelism: 4,
      prompt: "> ".to_string(),
      benchmark: 0,
    }
  }
}

pub fn cmd_parse() -> Options {
  let mut out = Options::default();

  let deflt_query = out.query.to_string();
  let deflt_lines = out.lines.to_string();
  let deflt_parallelism = out.parallelism.to_string();
  let deflt_prompt = out.prompt.to_string();
  let deflt_benchmark = out.benchmark.to_string();

  let long_about: String = format!("{}\n[{}]", DESCRIPTION, WEBSITE);

  let matches = Command::new(NAME)
    .version(VERSION)
    .about(DESCRIPTION)
    .long_about(long_about.as_ref())
    .arg(
      Arg::new("query")
        .short('q')
        .long("query")
        .value_name("QUERY")
        .default_value(&deflt_query)
        .help("Query string to search for"),
    )
    .arg(
      Arg::new("lines")
        .short('l')
        .long("lines")
        .value_name("LINES")
        .default_value(&deflt_lines)
        .help("Number of output lines to display"),
    )
    .arg(
      Arg::new("show-scores")
        .short('s')
        .long("show-scores")
        .help("Show numerical scores for each match"),
    )
    .arg(
      Arg::new("parallelism")
        .short('j')
        .long("parallelism")
        .value_name("THREADS")
        .default_value(&deflt_parallelism)
        .help("Maximum number of worker threads to use"),
    )
    .arg(
      Arg::new("prompt")
        .short('p')
        .long("prompt")
        .value_name("PROMPT")
        .default_value(&deflt_prompt)
        .help("Propmt to show when entering queries"),
    )
    .arg(
      Arg::new("benchmark")
        .short('b')
        .long("benchmark")
        .value_name("REPEATS")
        .default_value(&deflt_benchmark)
        .help("Set to a positive value to run that many repeated searches for benchmarking"),
    )
    .arg(
      Arg::new("workers")
        .long("workers")
        .value_name("THREADS")
        .help("Identical to \"--parallelism\""),
    )
    .arg(
      Arg::new("show-matches")
        .short('e')
        .long("show-matches")
        .value_name("QUERY")
        .help("Identical to \"--query\""),
    )
    .get_matches();

  out.query = if matches.is_present("query") {
    matches.value_of("query").unwrap().to_string()
  } else if matches.is_present("show-matches") {
    matches.value_of("show-matches").unwrap().to_string()
  } else {
    out.query
  };
  out.lines = matches
    .value_of("lines")
    .unwrap_or(&deflt_query)
    .parse()
    .unwrap_or(out.lines);
  out.show_scores = matches.is_present("show-scores");
  out.parallelism = {
    if matches.is_present("parallelism") {
      matches.value_of("parallelism").unwrap()
    } else if matches.is_present("workers") {
      matches.value_of("workers").unwrap()
    } else {
      &deflt_parallelism
    }
  }.parse()
    .unwrap_or(out.parallelism);
  out.prompt = matches
    .value_of("prompt")
    .unwrap_or(&out.prompt)
    .to_string();
  out.benchmark = matches
    .value_of("benchmark")
    .unwrap_or(&deflt_benchmark)
    .parse()
    .unwrap_or(out.benchmark);

  out
}
