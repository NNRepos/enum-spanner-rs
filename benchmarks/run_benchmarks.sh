#!/bin/sh

BENCHMARKS="DNA_arbitrary_distance DNA_growing_distance DNA_growing_fragment DNA_growing_length.json blog"

# Download DNA data
# ./download_dna.sh

# Download blgo data
# ./download_blog_data.sh

# create directory for results
mkdir results

# run benchmarks and store results
for B in $BENCHMARKS; do
  cargo run --release -- --benchmark-file $B.json > results/$B.json
done

# run some benchmarks with naive algorithm for comparison
for B in DNA_arbitrary_distance DNA_growing_distance; do
  cargo run --release -- --naive-quadratic --benchmark-file $B.json > results/$B-naive.json
done


