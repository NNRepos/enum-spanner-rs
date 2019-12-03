use std::fs::File;
use std::path::Path;
use std::io::prelude::*;
use std::time::Instant;
use super::mapping::{SpannerEnumerator,indexed_dag::{IndexedDag,TrimmingStrategy}};
use super::Algorithm;

use serde::{Deserialize, Serialize};

use super::regex;
use super::naive;

#[derive(Serialize, Deserialize, Clone)]
pub struct BenchmarkCase {
    name:     String,
    comment:  String,
    filename: String,
    regex:    String,
    jump: Option<usize>,
    trimming: Option<TrimmingStrategy>,
    length:   Option<u64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Delay {
    delay_min: f64,
    delay_max: f64,
    delay_avg: f64,
    delay_stddev: f64,
    delay_hist: Vec<u32>,
}

#[derive(Serialize, Deserialize)]
pub struct BenchmarkResult {
    benchmark: BenchmarkCase,
    num_states: usize,
    num_results: usize,
    width_avg: f64,
    width_max: usize,
    preprocess: f64,
    create_dag: Option<f64>,
    trim_dag: Option<f64>,
    index_dag: Option<f64>,
    enumerate: f64,
    delays: Option<Delay>,
    memory_usage: usize,
    memory_dag: usize,
    memory_matrices: usize,
    memory_jump_level: usize,
    memory_dag_max: usize,
    num_matrices: usize,
    num_used_matrices: usize,
    matrix_avg_size: f64,
    matrix_max_size: usize,
    matrix_avg_density: f64,
    num_levels: usize,
}

impl BenchmarkCase {
    pub fn read_from_file(filename: &Path) -> Result<Vec<BenchmarkCase>,Box<dyn std::error::Error>> {
        let mut input = String::new();

        File::open(&filename)?.read_to_string(&mut input)?;
        let path = filename.parent();

        let mut benchmarks: Vec<BenchmarkCase> = serde_json::from_str(&input)?;

        if let Some(path) = path {
            for mut benchmark in &mut benchmarks {
                benchmark.filename = path.join(benchmark.filename.clone()).to_str().unwrap().to_string();
            }
        }

        Ok(benchmarks)
    }

    pub fn new(name: String, comment: String, filename: String, regex: String, jump: usize, trimming: TrimmingStrategy) -> BenchmarkCase {
        BenchmarkCase {
            name,
            comment,
            filename,
            regex,
            length: None,
            jump: Some(jump),
            trimming: Some(trimming),
        }
    }

    fn measure_delays<'a>(&'a self, count_matches: usize, compiled_matches: &impl SpannerEnumerator<'a>, k: usize) -> Option<Delay> {
        if k == 0 {
            return None;
        }

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
            let delay: Vec<u32> = times.iter().map(|&d| {let i = ((d + 1000000000) - last) % 1000000000; last = d; i}).skip(1).collect();

            delays.push(delay);
        }

        let mut iters = Vec::with_capacity(k);
        for i in &delays {
            iters.push(i.iter());
        }

        let mut temp: Vec<u32> = Vec::with_capacity(k);

        let mean_delays: Vec<u32> = if count_matches == 0 {Vec::new()} else {
            (0..count_matches-1).map(|_| {
                temp.clear();
                for iter in &mut iters {
                    temp.push(*iter.next().unwrap());
                }

                *temp.iter().min().unwrap()
            }).collect()
        };

        let mean = stats::mean(mean_delays.iter().map(|&x| x));
        let stddev = stats::stddev(mean_delays.iter().map(|&x| x));
        let max: usize = *mean_delays.iter().max().unwrap_or(&0) as usize;
        let min = *mean_delays.iter().min().unwrap_or(&0);
        let mut hist = vec![0;max/1000 + 1];
        for &i in &mean_delays {
            hist[i as usize/1000]+=1;
        }

        Some(Delay {
            delay_min: min as f64 / 1000000000.0,
            delay_max: max as f64 / 1000000000.0,
            delay_avg: mean as f64 / 1000000000.0,
            delay_stddev: stddev as f64 / 1000000000.0,
            delay_hist: hist,
        })
    }

    fn measure<'a>(&'a self, enumerator: &mut impl SpannerEnumerator<'a>) -> (usize, f64, f64) {
        // Prepare the enumeration.
        let timer = Instant::now();
        enumerator.preprocess();
        let preprocess = timer.elapsed();

        // Count matches.
        let timer = Instant::now();
        let count_matches = enumerator.iter().count();
        let enumerate = timer.elapsed();

        (count_matches, preprocess.as_nanos() as f64 / 1000000000.0, enumerate.as_nanos() as f64 / 1000000000.0)
    }

    pub fn run(&self, algorithm: Algorithm, k: usize) -> Result<BenchmarkResult,std::io::Error> {
        let mut input = String::new();
        let trimming_strategy = match self.trimming {
            None => TrimmingStrategy::FullTrimming,
            Some(s) => s,
        };

        let jump_distance = match self.jump {
            None => 1,
            Some(d) => d,
        };

        // Read input file content.
        File::open(&self.filename)?.take(match self.length {
            Some(l) => l,
            None => std::u64::MAX,
        }).read_to_string(&mut input)?;

        // Compile the regex.
        let automaton = regex::compile(&self.regex);

        let num_states = automaton.get_nb_states();

        match algorithm {
            Algorithm::ICDT19 => {
                let mut enumerator = IndexedDag::new(automaton, &input, jump_distance, trimming_strategy, false);
                let (count_matches, preprocess, enumerate) = self.measure(&mut enumerator);
                let delays = self.measure_delays(count_matches, &enumerator, k);
                let (num_matrices, num_used_matrices, matrix_avg_size, matrix_max_size, matrix_avg_density, width_max, width_avg) = enumerator.get_statistics();
                let (create_dag, trim_dag, index_dag) = enumerator.get_times();
                let (dag_mem_max, dag_mem, matrices_mem, jump_level_mem) = enumerator.get_memory_usage();
                let num_levels = enumerator.num_levels();

                Ok(BenchmarkResult {
                    num_states,
                    benchmark: self.clone(),
                    num_results: count_matches,
                    num_matrices,
                    num_used_matrices,
                    matrix_avg_size,
                    matrix_max_size,
                    matrix_avg_density,
                    width_avg,
                    width_max,
                    preprocess,
                    enumerate,
                    memory_usage: dag_mem + matrices_mem + jump_level_mem,
                    memory_dag_max: dag_mem_max,
                    memory_dag: dag_mem,
                    memory_matrices: matrices_mem,
                    memory_jump_level: jump_level_mem,
                    num_levels,
                    create_dag: create_dag.map(|t| t.as_nanos() as f64/1000000000.0),
                    trim_dag: trim_dag.map(|t| t.as_nanos() as f64/1000000000.0),
                    index_dag: index_dag.map(|t| t.as_nanos() as f64/1000000000.0),
                    delays,
                })
            },
            Algorithm::Naive => {
                let mut enumerator = naive::naive::NaiveEnum::new(&automaton, &input);
                let (count_matches, preprocess, enumerate) = self.measure(&mut enumerator);
                let delays = self.measure_delays(count_matches, &enumerator, k);

                Ok(BenchmarkResult {
                    benchmark: self.clone(),
                    num_states: 0,
                    num_results: count_matches,
                    num_matrices: 0,
                    num_used_matrices: 0,
                    matrix_avg_size:0.0 ,
                    matrix_max_size: 0,
                    matrix_avg_density: 0.0,
                    width_avg: 0.0,
                    width_max: 0,
                    preprocess,
                    enumerate,
                    delays,
                    memory_usage: 0,
                    memory_dag_max: 0,
                    memory_dag: 0,
                    memory_matrices: 0,
                    memory_jump_level: 0,
                    num_levels: 0,
                    create_dag: None,
                    trim_dag: None,
                    index_dag: None,
                })
            },
            Algorithm::NaiveQuadratic => {
                let mut enumerator = naive::naive_quadratic::NaiveEnumQuadratic::new(&self.regex, &input);
                let (count_matches, preprocess, enumerate) = self.measure(&mut enumerator);
                let delays = self.measure_delays(count_matches, &enumerator, k);

                Ok(BenchmarkResult {
                    benchmark: self.clone(),
                    num_states: 0,
                    num_results: count_matches,
                    num_matrices: 0,
                    num_used_matrices: 0,
                    matrix_avg_size:0.0 ,
                    matrix_max_size: 0,
                    matrix_avg_density: 0.0,
                    width_avg: 0.0,
                    width_max: 0,
                    preprocess,
                    enumerate,
                    delays,
                    memory_usage: 0,
                    memory_dag_max: 0,
                    memory_dag: 0,
                    memory_matrices: 0,
                    memory_jump_level: 0,
                    num_levels: 0,
                    create_dag: None,
                    trim_dag: None,
                    index_dag: None,
                })
            },            
            Algorithm::NaiveCubic => {
                let mut enumerator = naive::naive_cubic::NaiveEnumCubic::new(&self.regex, &input).unwrap();
                let (count_matches, preprocess, enumerate) = self.measure(&mut enumerator);
                let delays = self.measure_delays(count_matches, &enumerator, k);

                Ok(BenchmarkResult {
                    benchmark: self.clone(),
                    num_states: 0,
                    num_results: count_matches,
                    num_matrices: 0,
                    num_used_matrices: 0,
                    matrix_avg_size:0.0 ,
                    matrix_max_size: 0,
                    matrix_avg_density: 0.0,
                    width_avg: 0.0,
                    width_max: 0,
                    preprocess,
                    enumerate,
                    delays,
                    memory_usage: 0,
                    memory_dag_max: 0,
                    memory_dag: 0,
                    memory_matrices: 0,
                    memory_jump_level: 0,
                    num_levels: 0,
                    create_dag: None,
                    trim_dag: None,
                    index_dag: None,
                })
            },            
        }

    }   
}



