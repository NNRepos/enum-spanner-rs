#!/bin/sh

# Download genome

# download genome, unzip, extract first chromosome and change all letters to 
# upper case

# It is expected that the download aborts after the first chromosome is extracted.
# There is no need to download the full genome.

# It is also expected that the file contains characters 'N', that correspond to
# a kind of placeholder
wget -O - ftp://ftp.ncbi.nlm.nih.gov/refseq/H_sapiens/annotation/GRCh38_latest/refseq_identifiers/GRCh38_latest_genomic.fna.gz 2>/dev/null | zcat | tr -d '\n' | tr acgt ACGT | tail -c +69 | head -c 248956422 > chromosome-1

