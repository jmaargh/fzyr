extern crate fzyr;

mod interactive;
mod opts;

use std::io;
use std::process;

use fzyr::config::SCORE_MIN;
use fzyr::search_score;

fn candidates_from_stdin() -> Vec<String> {
  let stdin = io::stdin();

  let mut out = Vec::new();
  let mut buff = String::with_capacity(opts::DEFLT_STRING_BUFFER_LEN);
  while let Ok(bytes) = stdin.read_line(&mut buff) {
    if bytes == 0 {
      break;
    }
    out.push(buff.clone());
    buff.clear();
  }

  out
}

fn to_slices<'src>(strings: &'src Vec<String>) -> Vec<&'src str> {
  strings.iter().map(|s| s.trim()).collect()
}

fn run() -> i32 {
  let options = opts::cmd_parse();

  if options.benchmark > 0 && options.query.is_empty() {
    println!("To benchmark, provide a query with one of the -q/-e/--query/--show-matches flags");
    return 1;
  }

  let candidates = candidates_from_stdin();
  let candidates = to_slices(&candidates);

  if options.benchmark > 0 {
    // Run a benchmarking run without output
    for _ in 0..options.benchmark {
      search_score(&options.query, &candidates, options.parallelism);
    }
    0
  } else if !options.query.is_empty() {
    // Run printing to stdout
    let results = search_score(&options.query, &candidates, options.parallelism);
    for result in results.iter().take(options.lines) {
      if options.show_scores {
        if result.score == SCORE_MIN {
          print!("(     ) ");
        } else {
          print!("({:5.2}) ", result.score);
        }
        println!("{}", result.candidate);
      }
    }
    0
  } else {
    // Run interactively
    interactive::run(&candidates, &options)
  }
}

fn main() {
  process::exit(run());
}
