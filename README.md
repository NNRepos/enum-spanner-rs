Constant-Delay Enumeration for Nondeterministic Document Spanners
=================================================================

This tool allows you to find efficiently all matches of a regular expression in
a string, i.e., find all contiguous substrings of the string that satisfy the
regular expression (including overlapping substrings).

**The tool is being actively developed and has not been thoroughly tested yet.
Use at your own risk.**

It is the reimplementation of
[the previous Python prototype](https://github.com/remi-dupre/enum-spanner/) and
was forked from
[the earlier version by Rémi Dupré](https://github.com/remi-dupre/enum-spanner-rs/).

Requirements
------------

It has been tested and developed using `rustc 1.34` and `cargo 1.34`. It will
not work with older Rust versions shipped by some Linux distributions, e.g.,
with version 1.32. You can check your rust version with `rustc --version`, and
install manually a more recent Rust version from the [Rust
website](https://www.rust-lang.org/learn/get-started).

Specific library requirements can be found in *Cargo.toml* and *Cargo.lock*.

Batch Usage
-----------

The program can be used to run some benchmarks in an automated fashion. Please
refer to the [benchmarks](https://github.com/PoDMR/enum-spanner-rs/tree/master/benchmarks)
folder for details.

Usage
-----

The program can also be run manually via Cargo by specifying a regular
expression with captures on the command line.

```bash
# Display all occurences of a pattern (regexp) in a file
cargo run --release -- [regexp] [file]
cat [file] | cargo run --release -- [regexp]

# For instance, this example will match 'aa@aa', 'aa@a', 'a@aa' and 'a@a'
echo "aa@aa" | cargo run --release -- ".+@.+"

# List optional parameters
cargo run -- --help

# Run unit tests
cargo test
```

The matches displayed correspond to all distincts substrings of the text that
match the given pattern. If the pattern contains named groups, the tool will
output one match for each possible assignment of the groups.

### Named groups

You can define named groups as follows: `(?P<group_a>a+)(?P<group_b>b+)`. This
example will extract any group of a's followed by a group of b's.

If no named group is specified, the algorithm will implicitly assume a group named `match` that captures the whole expression.

If a double underscore appears in a group name, the double underscore and evrything behing is stripped. This allows to workaround a limitation in rust regexp handling, where a group name has to be unique. To use the same group name several times just use a\_\_1, a\_\_2, etc.

Supported Syntax for Regular Expressions
----------------------------------------

The tool supports the same syntax as the Rust's regex crate, which is specified
[here](https://docs.rs/regex/1.1.6/regex/#syntax), except for **anchors, which
are not implemented yet**.

Underlying Algorithm
--------------------

The algorithm used by this tool is described in the research paper
*[Constant-Delay Enumeration for Nondeterministic Document
Spanners](https://arxiv.org/abs/1807.09320)*, by [Amarilli](https://a3nm.net/),
[Bourhis](http://cristal.univ-lille.fr/~bourhis/),
[Mengel](http://www.cril.univ-artois.fr/~mengel/) and
[Niewerth](http://www.theoinf.uni-bayreuth.de/en/team/niewerth_matthias/index.php).

It has been presented at the [ICDT'19](http://edbticdt2019.inesc-id.pt/)
conference.

The tool will first compile the regular expression into a non-deterministic
finite automaton, and then apply an *enumeration algorithm*. Specifically, it
will first pre-process the string (without producing any matches), in time
linear in the string and polynomial in the regular expression. After this
pre-computation, the algorithm produces the matches sequentially, with constant
*delay* between each match.
