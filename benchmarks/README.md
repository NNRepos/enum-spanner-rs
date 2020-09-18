Benchmarks
==========

The json-files in this folder are predefined benchmarks that were used to test
the performance of this implementation. This readme explains how to download the
data needed for the example benchmarks, how to run the benchmarks, the format of the
json files describing the benchmarks, the format of the json objects describing the benchmark results, and how to extract data from the benchmark results.


Tutorial
========
Download the data necessary for the benchmarks:
```bash
# download dna data
./download_dna.sh

# download blog data
./download_blog_data.sh
```

This should create files chromosome-1 (size 248.956.422 bytes) and blog-corpus-ascii (size 803.619.661). 
The scripts need standard unix utilities (wget, gzip, unzip, head, tail, tr) to extract and format the data.
If there are problems see below for details on how to prepare the data manually.

Run benchmarks. This will several hours in total:
```bash
./run_benchmarks.sh
```

If all goes well, this will create a results folder, with the output from the benchmarks. For details on the format see below.



Detailed Explanations
=====================


Downloading and Preparing the Data
----------------------------------
Most of the benchmarks need additional data. All benchmarks prefixed with dna require
DNA data (strings using the alphabet ACGT). We provide a small file with example data.
Our benchmark results used the first chromosome of the human reference genome [GRCh38](https://www.ncbi.nlm.nih.gov/genome/guide/human/). In order to directly apply our benchmarks, the data needs to be unzipped, the line breaks need to be removed and all characters have to be chnaged to uppercase. The script download\_dna.sh does all these steps on a Linux machine. 

Similar results can be achieved by creating the file chromosome-1 with a 250 MB random string over the alphabet ACGT. 

The blog benchmarks are described in the [master thesis of Andrea Morciano](https://www.politesi.polimi.it/bitstream/10589/135034/1/2017_07_Morciano.pdf). The blog corpus can be found [here](http://u.cs.biu.ac.il/~koppel/BlogCorpus.htm). It needs to be extracted, all files concateneted into one file blog-corpus, and invalid unicode encodings needs to be repaired or removed. As the dictionary mathcers in the queries do not use any non-ascii symbols, one can simply remove all non-ascii characters. The process is automated in the script download\_blog\_data.sh

List of Benchmarks
------------------
| file name | Regex | description |
| --------- | ----- | ----------- |
| DNA\_arbitrary\_distance.json | TTAC.\*CACC       | Search for two DNA patterns in arbitrary distance |
| DNA\_growing\_distance.json   | TTAC.{0,k}CACC    | Search for two DNA patterns with distance at most k (10 <= k <= 10000) |
| DNA\_growing\_fragment.json   | w1.{0,1000}w2     | Search for two DNA patterns w1 and w2 with distance at most 1000. The lengths of the fragments varies.|
| DNA\_growing\_length.json     | TTAC.{0,1000}CACC | Search for two DNA patterns with distance 1000. The length of the input string varies. |
| blog.json | | Queries from the master thesis of Andrea Morciano |


RAM-Usage
---------
The final size of our data structure depends a lot on the size of the automaton and the number of results. However, during preprocessing we create a BITMAP of which states in the product from the automaton and the document are reachable. By construction its size in bits is number of states of the automatom times size of the input. The benchmark DNA\_growing\_length needs roughly 30 GB of free RAM to run. All other benchmarks use at most a few GB.

Usage
-----
```bash
# Run all benchmarks described in [file]
cargo run --release -- --benchmark-file [file] 
```

The statistics are written to stdout. You probably want to redirect the output to a file for further processing.


Format
------
The json file consists of a set of benchmark objects, with the following fields:

| field | description |
| ----- | ----------- |
| name | used for reference purposes and has no impact on the benchmark |
| comment | allows for a human readbale description. has no impact on benchmark |
| filename | filename of the input document |
| regex | regular expression, i.e., the query |
| trimming | Whether the DAG is trimmed or not |
| length | Optional. If present only the first n bytes of the input file are used |
  
The possible values for trimming are currently only FullTrimming and NoTrimming.

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
| num\_levels | number of levels that are in hte image of the jump function |

All times are given in seconds, all memory allocations in bytes. This is not the actual amount of memory needed, but a sum over the allocations made. It does not include stack, program code, or overhead of the allocator. Also the space requirements are for the final data structure. Right now, additional memory is needed to store the input string in memory and to represent the non-trimmed DAG. Especially the latter can be of considerable size, as it uses number of states in the automaton times length of the input string many bits.

The detailed analysis of delays is only available if the optional --repetitions <num> parameter is used. The parameter gives the number of times, the enumeration part should be performed. During each path, every delay is stored in memory. After <num> passes, for every produced results, there are <num> delay measurements. We take the median of these <num> measurements to compute the statistics in the table below. If there is only one repetition, there will be some outliers, e.g., due to interrupt processing. Note that delays due to interrupts can be several order of magnitude larger than all delays encoutered due to the algorithm. Thus to evaluate the algorithm (and not the whole system performance), there should be a few repetitions. For our own analysis we took 10 repetitions, but your mileage may vary.

If there are many results, collecting these statistics requires a considerable amount of RAM.

This are the fields of the delay object:
| field | description |
| ----- | ----------- |
| delay\_min | minimal time between two results |
| delay\_max | maximal time between two results |
| delay\_avg | average time between two results |
| delay\_hist | delay histogram (see explanation below) |

The histogram field contains an array, where the first entry corresponds to how many results had a delay (measured from the output of the previous results) smaller than one microsecond. The next entry says how many results had a delay betweeen one and two microseconds and so on.

Extracting Data
---------------
Data can be extracted from the output either manually or with JSON query tools. 
The tool [jq](https://stedolan.github.io/jq/) can be used from the command line.

Example: extract pairs of document length and preprocessing speed (in bytes/s):
```bash
jq '[.[] | [.benchmark.length,.benchmark.length/.preprocess]]'
```

