mod automaton;
mod benchmark;
mod mapping;
mod matrix;
mod progress;
mod regex;

extern crate clap;
extern crate regex as lib_regex;
extern crate regex_syntax;
extern crate bit_vec;

use std::fs::File;
use std::io::prelude::*;
use std::io::{stdin, stdout};
use std::time;
use std::path::Path;

use clap::{App, Arg};
use mapping::Mapping;
use mapping::indexed_dag::TrimmingStrategy;
use benchmark::BenchmarkCase;

#[derive(PartialEq, Eq)]
enum DisplayFormat {
    /// Only display the count of matches
    Count,
    /// Display in the re-compare format: https://github.com/gchase/re-compare
    CompareFormat,
    /// Human-readable format
    Verbose { show_offset: bool },
}

fn main() {
    //  ____
    // |  _ \ __ _ _ __ ___  ___ _ __
    // | |_) / _` | '__/ __|/ _ \ '__|
    // |  __/ (_| | |  \__ \  __/ |
    // |_|   \__,_|_|  |___/\___|_|
    //
    let matches = App::new("Enumerate matchings")
        .version("0.1")
        .author("Rémi Dupré <remi.dupre@ens-paris-saclay.fr>\n\
		         Matthias Niewerth <matthias.niewerth@uni-bayreuth.de>")
        .about("Enumerate all matches of a regular document spanner on a text.")
        .arg(
            Arg::with_name("benchmark")
                .long("benchmark")
                .help("Output statistics. Requiers one of benchmark-file or file to be present.")
        )
        .arg(
            Arg::with_name("benchmark-file")
                .long("benchmark-file")
                .help("Read a set of benchmarks from a file in JSON syntax. Implies --benchmark")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("regex")
                .help("The pattern to look for.")
                .required(true)
                .conflicts_with("benchmark-file"),
        )
        .arg(
            Arg::with_name("file")
                .help("The file to be read, if none is specified, STDIN is used.")
        )
        .arg(
            Arg::with_name("count")
                .short("c")
                .long("count")
                .help("Display the number of matches instead."),
        )
        .arg(
            Arg::with_name("bytes_offset")
                .short("b")
                .long("bytes-offset")
                .help("Print the 0-based offset of each matching part and groups."),
        )
        .arg(Arg::with_name("compare")
                .long("compare")
                .help("Output matches in a format suitable with re-compare: \
                       https://github.com/gchase/re-compare")
        )
        .arg(
            Arg::with_name("use_naive")
                .long("naive")
                .help("Use a naive algorithm to equivalently print all matches"),
        )
        .arg(
            Arg::with_name("use_naive_cubic")
                .long("naive-cubic")
                .help("Use a naive algorithm to enumerate all subwords that match the input regex. \
                       This algorithm runs in time O(|text|³ + exp(|regex|))"),
        )
        .arg(
            Arg::with_name("use_naive_quadratic")
                .long("naive-quadratic")
                .help("Use a naive algorithm to enumerate all subwords that match the input regex. \
                       This algorithm runs in time O(|regex||text|²)"),
        )
        .arg(
            Arg::with_name("debug_infos")
                .short("i")
                .long("debug-infos")
                .help("Display debuging infos"),
        )
		.arg(
			Arg::with_name("jump_distance")
			    .long("jump-distance")
                .short("j")
                .takes_value(true)
                .help("Distance between jump target. This affects the number of matrices computed and \
                       is a trade-off between pre-processing and enumeration time. Bigger values mean \
                       faster preprocessing and possibly slower enumeration."),
		)
        .arg(
            Arg::with_name("trimming_strategy")
            .long("trimming")
            .short("t")
            .takes_value(true)
            .default_value("full")
            .possible_value("full")
            .possible_value("partial")
            .possible_value("no")
            .help("Should the DAG be trimmed? Useful for benchmarking the effect of trimming."),
        )
        .arg(
            Arg::with_name("repetitions")
            .long("repetitions")
            .takes_value(true)
            .default_value("0")
            .help("Enables a detailed delay analysis if >0. The parameter gives the number of repetitions used to filter outliers."),
        )
        .get_matches();

    // Extract parameters
    let benchmark = matches.is_present("benchmark");
    let repetitions = match matches.value_of("repetitions") {
        None => 0,
        Some(s) => {
	        match s.parse::<usize>() {
		        Ok(n) => n,
                Err(_) => panic!("Not a number: {}", s),
	        }
        }
    };
    let count = matches.is_present("count");
    let show_offset = matches.is_present("bytes_offset");
    let compare_format = matches.is_present("compare");

    let use_naive = matches.is_present("use_naive");
    let use_naive_cubic = matches.is_present("use_naive_cubic");
    let use_naive_quadratic = matches.is_present("use_naive_quadratic");

    let debug_infos = matches.is_present("debug_infos");

    let trimming_strategy_str = matches.value_of("trimming_strategy");
    let trimming_strategy = match trimming_strategy_str {
        None => TrimmingStrategy::FullTrimming,
        Some("full") => TrimmingStrategy::FullTrimming,
        Some("partial") => TrimmingStrategy::PartialTrimming,
        Some("no") => TrimmingStrategy::NoTrimming,
        Some(s) => panic!("Invalid option for trimming: {}", s),
    };

    let jump_distance_str = matches.value_of("jump_distance");
    let jump_distance = match jump_distance_str {
	    None => 1,
        Some(s) => {
	        match s.parse::<usize>() {
		        Ok(n) => n,
                Err(_) => panic!("Not a number: {}", s),
	        }
        }
    };

    let display_format = match (count, compare_format, show_offset) {
        (true, _, _) => DisplayFormat::Count,
        (_, true, _) => DisplayFormat::CompareFormat,
        _ => DisplayFormat::Verbose { show_offset },
    };

    //  ____                  _                          _
    // | __ )  ___ _ __   ___| |__  _ __ ___   __ _ _ __| | __
    // |  _ \ / _ \ '_ \ / __| '_ \| '_ ` _ \ / _` | '__| |/ /
    // | |_) |  __/ | | | (__| | | | | | | | | (_| | |  |   <
    // |____/ \___|_| |_|\___|_| |_|_| |_| |_|\__,_|_|  |_|\_\
    //

    let benchmark_file = matches.value_of("benchmark-file");

    if benchmark_file != None {
        print!("[");
        let path = Path::new(benchmark_file.unwrap());
        let benchmarks = benchmark::BenchmarkCase::read_from_file(&path).unwrap();
        let mut first = true;
        for benchmark in benchmarks {
            let result;
            println!("{}", if first {""} else {","});
            if use_naive_quadratic {
                result = benchmark.run_quadratic().unwrap();
            } else {
                result = benchmark.run(repetitions).unwrap();
            }
            print!("{}", serde_json::to_string_pretty(&result).unwrap());
            first = false;
        }
        println!("\n]");
        return;
    }

    let regex_str = matches.value_of("regex").unwrap();

    if benchmark {
        let benchmark_case = BenchmarkCase::new("CLI Benchmark".to_string(), "Benchmark invoked by CLI.".to_string(), matches.value_of("file").unwrap().to_string(), regex_str.to_string(), jump_distance, trimming_strategy);
        let result = if use_naive_quadratic {
            benchmark_case.run_quadratic().unwrap()
            } else {
            benchmark_case.run(repetitions).unwrap()
        };
            
        print!("{}", serde_json::to_string_pretty(&result).unwrap());

        return;
    }


    //  ___                   _
    // |_ _|_ __  _ __  _   _| |_ ___
    //  | || '_ \| '_ \| | | | __/ __|
    //  | || | | | |_) | |_| | |_\__ \
    // |___|_| |_| .__/ \__,_|\__|___/
    //           |_|

    // Read the text
    let mut text = String::new();
    match matches.value_of("file") {
        Some(filename) => {
            let mut file = File::open(filename).unwrap();
            file.read_to_string(&mut text).unwrap()
        }
        None => stdin().read_to_string(&mut text).unwrap(),
    };

    // Remove trailing newlines
    while text.as_bytes().last() == Some(&b'\n') {
        text.pop();
    }

    //  __  __       _       _
    // |  \/  | __ _| |_ ___| |__
    // | |\/| |/ _` | __/ __| '_ \
    // | |  | | (_| | || (__| | | |
    // |_|  |_|\__,_|\__\___|_| |_|
    //

    let regex = regex::compile(regex_str);
    regex
        .render("automaton.dot")
        .expect("Could not create the dotfile.");

    let timer = time::Instant::now();

    fn handle_matches<'t>(
        matches: impl Iterator<Item = mapping::Mapping<'t>>,
        text: &str,
        timer: &time::Instant,
        display_format: DisplayFormat,
    ) {
        match display_format {
            DisplayFormat::Count => {
                let count = matches.count();
                println!("{}", count)
            }
            DisplayFormat::CompareFormat => {
                for mapping in matches {
                    let span = mapping
                        .main_span()
                        .expect("A mapping should never be empty");

                    println!(
                        r#">>>>{{"match": {:?}, "span": [{},{}], "time": {}}}"#,
                        &text[span.clone()],
                        span.start,
                        span.end,
                        timer.elapsed().as_millis()
                    )
                }

                println!(
                    r#">>>>{{"match": "EOF", "span": [-1,-1], "time": {}}}"#,
                    timer.elapsed().as_millis()
                );
            }
            DisplayFormat::Verbose { show_offset } => {
                for (count, mapping) in matches.enumerate() {
                    print!("{} -", count + 1);

                    if show_offset {
                        for (name, range) in mapping.iter_groups() {
                            print!(" {}:{},{}", name, range.start, range.end);
                        }
                    } else {
                        for (name, text) in mapping.iter_groups_text() {
                            print!(" {}:{:?}", name, text);
                        }
                    }

                    println!();
                }
            }
        }
    }

    let indexed_dag;

    let iter_matches:Box<Iterator<Item=Mapping>> = if use_naive {
        Box::new(mapping::naive::NaiveEnum::new(&regex, &text))
    } else if use_naive_cubic {
        Box::new(regex::naive::NaiveEnumCubic::new(regex_str, &text).unwrap())
    } else if use_naive_quadratic {
        Box::new(regex::naive::NaiveEnumQuadratic::new(regex_str, &text))
    } else {
        indexed_dag=regex::compile_matches_progress(regex, &text, jump_distance, trimming_strategy);
        Box::new(indexed_dag.iter())
    };

    handle_matches(iter_matches, &text, &timer, display_format);

    //  ____       _                   ___        __
    // |  _ \  ___| |__  _   _  __ _  |_ _|_ __  / _| ___  ___
    // | | | |/ _ \ '_ \| | | |/ _` |  | || '_ \| |_ / _ \/ __|
    // | |_| |  __/ |_) | |_| | (_| |  | || | | |  _| (_) \__ \
    // |____/ \___|_.__/ \__,_|\__, | |___|_| |_|_|  \___/|___/
    //                         |___/

    if debug_infos {
        eprintln!("===== Debug Infos =====");
        // eprintln!(" - Levels count: {}", compiled_matches.get_nb_levels());
    }

    std::process::exit(0);
}
