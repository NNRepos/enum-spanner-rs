use std::fs::File;
use std::io::prelude::*;
use std::time::Instant;

use super::regex;

struct BenchmarkCase {
    name:     &'static str,
    comment:  &'static str,
    filename: &'static str,
    regex:    &'static str,
}

pub fn run_all_tests<T>(stream: &mut T) -> Result<(), std::io::Error>
where
    T: std::io::Write,
{
    if cfg!(debug_assertions) {
        eprintln!("[WARNING]    Running benchmarks in debug mode.");
    }

    let benchmarks = vec![
        BenchmarkCase {
            name:     "First columns of CSV",
            comment:  "Extract the first three columns of the input CSV document.",
            filename: "benchmarks/pablo_alto_trees.csv",
            regex:    r"\n(?P<x>[^,]+),(?P<y>[^,]+),(?P<z>[^,]+),",
        },
        BenchmarkCase {
            name:     "Pairs of words",
            comment:  "Extract all pairs of words that are in the same sentence.",
            filename: "benchmarks/lorem_ipsum.txt",
            regex:    r"[^\w](?P<word1>\w+)[^\w]((.|\n)*[^\w])?(?P<word2>\w+)[^\w]",
        },
        BenchmarkCase {
            name:     "Close DNA",
            comment:  "Find two substrings of a DNA sequence that are close from one another.",
            filename: "benchmarks/dna.txt",
            regex:    r"TTAC.{0,1000}CACC",
        },
        BenchmarkCase {
            name:     "All substrings",
            comment:  "Extract all non-empty substrings from the input document.",
            filename: "benchmarks/lorem_ipsum.txt",
            regex:    r"(.|\n)+",
        },
    ];

    for benchmark in benchmarks {
        let mut input = String::new();

        write!(stream, "-- {} ---------------\n", benchmark.name)?;
        write!(stream, "{}\n", benchmark.comment)?;

        // Read input file content.
        write!(stream, " - Loading file content ... ")?;
        stream.flush()?;
        let timer = Instant::now();

        File::open(benchmark.filename)?.read_to_string(&mut input)?;

        write!(
            stream,
            "{:.2?}\t({} bytes)\n",
            timer.elapsed(),
            input.as_bytes().len()
        )?;

        // Run the test itself.
        run_test(stream, benchmark.regex, input)?;

        write!(stream, "\n")?;
    }

    Ok(())
}

/// Compute time spent on running the regex over the given input file.
fn run_test<T>(stream: &mut T, regex: &str, input: String) -> Result<(), std::io::Error>
where
    T: std::io::Write,
{
    // Compile the regex.
    write!(stream, " - Compiling regex      ... ")?;
    stream.flush()?;
    let timer = Instant::now();

    let regex = regex::compile(regex);

    write!(
        stream,
        "{:.2?}\t({} states)\n",
        timer.elapsed(),
        regex.get_nb_states()
    )?;

    // Prepare the enumeration.
    write!(stream, " - Compiling matches    ... ")?;
    stream.flush()?;
    let timer = Instant::now();

    let compiled_matches = regex::compile_matches(regex, &input);

    write!(
        stream,
        "{:.2?}\n",
        timer.elapsed()
    )?;

    // Enumerate matches.
    write!(stream, " - Enumerate matches    ... ")?;
    stream.flush()?;
    let timer = Instant::now();

    let count_matches = compiled_matches.iter().count();

    write!(
        stream,
        "{:.2?}\t({} matches)\n",
        timer.elapsed(),
        count_matches
    )?;

    Ok(())
}
