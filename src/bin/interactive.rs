extern crate console;

use io;
use std::io::Write;

use self::console::{Key, Style, Term};

use fzyr::config::SCORE_MIN;
use fzyr::{search_locate, LocateResult, LocateResults};

use super::opts;

pub fn run(candidates: &[&str], options: &opts::Options) -> i32 {
  let mut terminal = Terminal::new(&options.prompt, options.show_scores, options.lines);

  if let Err(_) = terminal.run(candidates, options.parallelism) {
    eprintln!("Failed to write to stdout");
    1
  } else {
    0
  }
}

struct Terminal<'a> {
  result_count: usize,
  max_display_width: usize,
  prompt: &'a str,
  show_scores: bool,
  drawn_lines: usize,
  term: Term,
  standout: Style,
}

impl<'a> Terminal<'a> {
  fn new(prompt: &'a str, show_scores: bool, max_results: usize) -> Self {
    let term = Term::stdout();
    let size = term.size();
    Self {
      result_count: max_results.min((size.0 as usize).saturating_sub(1)),
      max_display_width: size.1 as usize,
      prompt: prompt,
      show_scores: show_scores,
      drawn_lines: 0,
      term: term,
      standout: Style::new().reverse(),
    }
  }
}

impl<'a> Terminal<'a> {
  fn run(&mut self, candidates: &[&str], parallelism: usize) -> io::Result<()> {
    let mut query = String::with_capacity(opts::DEFLT_STRING_BUFFER_LEN);

    let mut should_search = true;
    loop {
      if should_search {
        self.draw(&query, &search_locate(&query, candidates, parallelism))?;
      }

      should_search = match self.term.read_key()? {
        Key::Char(ch) if ch == '\u{08}' || ch == '\u{7f}' => match query.pop() {
          // Backspace or delete
          Some(_) => true,
          None => false,
        },
        Key::Char(ch) => {
          query.push(ch);
          true
        }
        _ => false,
      };
    }
  }

  fn draw(&mut self, query: &str, results: &LocateResults) -> io::Result<()> {
    self.clear()?;
    self.draw_query(query)?;
    self.draw_results(results)?;
    Ok(())
  }

  fn clear(&mut self) -> io::Result<()> {
    self.term.clear_line()?;
    self.term.clear_last_lines(if self.drawn_lines > 1 {
      self.drawn_lines.checked_sub(1).unwrap_or(0)
    } else {
      self.drawn_lines
    })?;
    self.drawn_lines = 0;
    Ok(())
  }

  fn draw_query(&mut self, query: &str) -> io::Result<()> {
    writeln!(
      self.term,
      "{}{}{}",
      self.prompt,
      query,
      self.standout.apply_to(" "),
    )?;
    self.drawn_lines += 1;
    Ok(())
  }

  fn draw_results(&mut self, results: &LocateResults) -> io::Result<()> {
    // Write the results
    let total_results = results.len().min(self.result_count);
    let mut line_count: usize = 0;
    for result in results.iter().take(total_results) {
      if line_count > 0 {
        self.term.write_line("")?;
      }
      self.draw_result(result)?;
      line_count += 1;
      self.drawn_lines += 1;
    }

    // Write empty lines for the rest
    while line_count < total_results {
      self.draw_empty()?;
      line_count += 1;
      self.drawn_lines += 1;
    }

    Ok(())
  }

  fn draw_empty(&mut self) -> io::Result<()> {
    self.term.write_line("")
  }

  fn draw_result(&mut self, result: &LocateResult) -> io::Result<()> {
    let mut spent_width = 0;

    if self.show_scores {
      if result.score == SCORE_MIN {
        write!(self.term, "(     ) ")?;
      } else {
        write!(self.term, "({:5.2}) ", result.score)?;
      }
      spent_width += 8;
    }

    for (i, ch) in result
      .candidate
      .chars()
      .take(self.max_display_width - spent_width)
      .enumerate()
    {
      if result.match_mask[i] {
        write!(self.term, "{}", self.standout.apply_to(ch))?;
      } else {
        write!(self.term, "{}", ch)?;
      }
    }

    Ok(())
  }
}
