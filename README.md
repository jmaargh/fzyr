# fzyr

**fzyr** is a simple and fast fuzzy text search. It exists as both a Rust library and a standalone executable.

Basically [fzy](https://github.com/jhawthorn/fzy) re-written in [Rust](https://www.rust-lang.org/).

## Why?

`fzyr` exists because I wanted a fuzzy finder library while learning Rust. However, you may find that it useful for your purposes

`fzyr` is very similar to `fzy`, so inherits its advantages (at least as of Aug 2018). For most purposes it should be usable as a drop-in replacement.

Advantages over `fzy`:

+ It's works on Windows! Or at least it should, that's not actually been tested yet, let me know if it doesn't 🖥

+ It works with all unicode strings! Hello, rest of the world 🗺️

+ You can easily install with [Cargo](https://doc.rust-lang.org/stable/cargo/)! Cross-platform package management 📦

+ It's a Rust library! Use the algorithm in your own projects 😀

Disadvantages over `fzy`:

+ It's less-well tested

+ It doesn't support arbitrary tty i/o (only stdin/stdout)

## Installation

# [Cargo](https://doc.rust-lang.org/stable/cargo/)

    cargo install fzyr

# Ubuntu

Deb coming soon...

# Homebrew

Might arrive at some point...

# Windows

Use Cargo

## Usage

Check out [fzy](https://github.com/jhawthorn/fzy#usage) for some usage examples.

To search for lines containing "something" in a file:

    $ cat very-long-file | fzyr -q something

To search interactively for a file:

    $ find . -type f | fzyr

Explore the options with:

    $ fzyr -h

## Library documentation

Coming soon...

## Algorithm

The alorithm is near-identical to that of `fzy`. That means:

+ Search is case-insensitive (all characters are converted to their unicode-defined lowercase version, if one exists)

+ Results must contain the entire query string, in the right order, but without the letters necessarily being consecutive

+ Results are all given a numerical score, and returned in best-score-first order

+ Prefers consecutive characters and characters that start words/filenames

+ Prefers shorter results
