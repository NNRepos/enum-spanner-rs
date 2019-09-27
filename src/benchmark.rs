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
        BenchmarkCase {
            name:     "Complex DNA",
            comment:  "Complex DNA query that exploits flashlight search.",
            filename: "benchmarks/dna.txt",
            regex:    r"C.{0,15}(?P<x>T).{0,15}(?P<y>G*).{0,15}(?P<z>C).{0,15}A",
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

/// get detailed statistics on the delay.
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

    let compiled_matches = regex::compile_matches(regex, &input, 1);

    write!(
        stream,
        "{:.2?}\n",
        timer.elapsed()
    )?;

    // Count matches.
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
    
    let k=20;
    let mut delays = Vec::with_capacity(k);
    // Do k iterations to get rid of outliers
    for _ in 0..k {
        let start_time = Instant::now();
        let mut times = Vec::with_capacity(count_matches);
        let _ = compiled_matches.iter().map(|x| {
            times.push(start_time.elapsed().subsec_nanos());

            x
        }).count();

        let mut last = 0;
        let delay: Vec<u32> = times.iter().map(|&d| {let mut i = ((d + 1000000000) - last) % 1000000000; last = d; i}).skip(1).collect();
                    
        delays.push(delay);
    }

    let mut iters = Vec::with_capacity(k);
    for i in &delays {
        iters.push(i.iter());
    }

    let mut temp: Vec<u32> = Vec::with_capacity(k);

    let mean_delays: Vec<u32> = (0..count_matches-1).map(|_| {
        temp.clear();
        for mut iter in &mut iters {
            temp.push(*iter.next().unwrap());
        }

        temp.sort();

        (temp[6] + temp[7] + temp[8] + temp[9] + temp[10] + temp[11])/6
    }).collect();

    let mean = stats::mean(mean_delays.iter().map(|&x| x));
    let stddev = stats::stddev(mean_delays.iter().map(|&x| x));
    let max: usize = *mean_delays.iter().max().unwrap() as usize;
    let min = mean_delays.iter().min().unwrap();
    writeln!(stream,"Statistics: {} {} {} {}", min, mean, max, stddev)?;
    let mut hist = vec![0;max/1000 + 1];
    for &i in &mean_delays {
        hist[i as usize/1000]+=1;
    }
    
    writeln!(stream,"Histogramm:\n{:?}", hist)?;

    writeln!(stream,"Outliers:")?;

    for (i,d) in mean_delays.iter().enumerate().filter(|(_,&x)| x>50000) {
        writeln!(stream,"{} took {} usec", i, d/1000)?;
    }

    Ok(())
}




