Benchmarks
==========

The json-files in this folder are predefined benchmarks that were used to test
the performance of this implementation.

Requirements
------------
Most of the benchmarks need additional data. All benchmarks prefixed with dna require
DNA data (strings using the alphabet ACGT). We provide a small file with example data.
Our benchmark results used the first chromosome of the human genome TODO:link.

Similar results can be achieved by just using 2500 copies of dna.txt in a single file (for the largest benchmarks).

The blog benchmarks are TODO

RAM-Usage
---------
The final size of our data structure depends a lot on the size of the automaton and the number of results. However, during preprocessing we create a BITMAP of which states in the product from the automaton and the document are reachable. By construction its size in bits is number of states of the automatom times size of the input. The largests benchmarks need roughly 30 GB of free RAM to run.

Usage
-----
```bash
# Run all benchmarks described in [file]
cargo run --release -- --benchmark-file [file] 
```

The statistics are written to stdout. You probably want to redirect the output to a file.


Format
------
The json file consists of a set of benchmark objects, each object containts the
strings name and comment (these have no impact on the algorithm), the filename (relative filenames are evaluated according to the cwd) containing the document, and the regex formula. Optionally they can contain a length, in wich case the query is restricted to the first length bytes and a trimming attribute that controls whether the DAG shall be trimmed prior to enumeration or not.

The output format likewise contains a set of benchmark-result objects. Each of these contain the processed benchmark object (for reference) and a bunch of statistics.
The meaning of the fields are:

| field | description |
| ----- | ----------- |
| num\_results | total number of results |
| width\_avg | average number of states per level in trimmed DAG |
| width\_max | maximum number of states in one lvel in trimmed DAG |
| compile\_regex | time to parse regex and translate it into an automaton |
| preprocess | total time spent in preprocessing |
| create\_dag | time spent computing all reachable states in DAG |
| trim\_dag | time spent trimming the DAG |
| index\_dag| time spent computing the reachability index |
| enumerate | total time for enumeration |
| delay | detailes analysis of delays (see below) |
| memory\_usage | total memory allocated in the final index structure |
| memory\_dag | memory to represent the DAG in final index structure |
| memory\_matrices | memory allocated for reachability matrices |
| memory\_jump\_level | memory allocated for the jump level function |
| num\_matrices | total number of stored matrices |
| matrix\_avg\_size | average matrix size (width \* height) |
| matrix\_max\_size | maximal matrix size (width \* height) |
| matrix\_avg\_density | TODO |
| num\_levels | number of levels that are in hte image of the jump function |

All times are given in seconds, all memory allocations in bytes. This is not the actual amount of memory needed, but a sum over the allocations made. It does not include stack, program code, or overhead of the allocator. Also the space requirements are for the final data structure. Right now, additional memory is needed to store the input string in memory and to represent the non-trimmed DAG. Especially the latter can be of considerable size, as it uses number of states in the automaton times length of the input string many bits.

The detailed analysis of delays is only available if the optional --repetitions <num> parameter is used. The parameter gives the numebr of times, the enumeration part should be performed. During each path, every delay is stored in memory. After <num> passes, for every produced results, there are <num> delay measurements, where the median is taken. Using this median, the following statistics are computed. If there is only one repetition, there will be some outliers, e.g., due to interrupt processing. Note that delays due to interrupts can be several order of magnitude larger than all delays encoutered due to the algorithm. Thus to evaluate the algorithm (and not the whole system performance), there should be a few repetitions. For our own analysis we took 10 repetitions, but your mileage may vary.

If there are many results, collecting this statistics can use a considerable amount of RAM.

| field | description |
| ----- | ----------- |
| delay\_min | minimal time between two results |
| delay\_max | maximal time between two results |
| delay\_avg | average time between two results |
| delay\_hist | delay histogram (see explanation below) |

The histogram field contains an array, where the first entry corresponds to how many results hat a delay (measured from the output of the previous results) smaller than one microsecond. The next entry says how many results had a delay betweeen one and two microseconds and so on.


