#!/bin/bash

# graph files
name=germany
graph=../${name}_ch.fmi
stgt_graph=../stgtregbz_ch.fmi

mkdir -p analysis
mkdir -p analysis/img

## quadtree analysis
if ! test -f "analysis/${name}_qt.csv"; then
    cargo run --release --bin quadtree_analysis -- $graph 1 15 > analysis/${name}_qt.csv
fi


## wspd analysis
wspd="cargo run --release --bin wspd_analysis -- -g $graph"
wspd_analysis() {
    local d=$1
    local e=$2
    local outfile="analysis/${name}_d${d}_e${e:2}.txt"
    if ! test -f "$outfile"; then
        eval $wspd -d $d -e $e --geom_check 0.01 > $outfile
    fi
}

# wspd analysis, without geometric error check
wspd_analysis_sizes() {
    local d=$1
    local e=$2
    local outfile="analysis/${name}_d${d}_e${e:2}.txt"
    if ! test -f "$outfile"; then
        eval $wspd -d $d -e $e > $outfile
    fi
}

for d in 5
do
    for e in 0.9 0.8 0.7 0.6 0.5 0.4 0.3 0.25 0.2 0.15 0.1 0.09 0.08 0.07 0.06 0.05 0.04 0.03 0.025
    do
        wspd_analysis $d $e
    done
done

for d in 5 8 9 10 11 12
do
    outfile="analysis/${name}_d${d}_e10.txt"
    if ! test -f "$outfile"; then
        eval $wspd -d $d -e 1 --geom_check 0.01 > $outfile
    fi
done

for e in 0.9 0.8 0.7 0.6 0.5 0.4 0.3 0.25 0.2 0.15 0.1 0.075 0.05 0.01
do
    for d in 8 9 10 11 12
    do
        wspd_analysis $d $e
    done
done

wspd_analysis_sizes 8 0.06
wspd_analysis_sizes 11 0.45


## hitting set data gen

wspd_generate_data() {
    local d=$1
    local e=$2
    local outfile="out/${name}_paths_d${d}_e${e:2}.txt"
    local wspd_gen="cargo run --release --bin wspd -- -g $graph -o $outfile"
    if ! test -f "$outfile"; then
        eval $wspd_gen -d $d -e $e
    fi
}

wspd_generate_data 8 0.06
wspd_generate_data 9 0.15
wspd_generate_data 10 0.25
wspd_generate_data 11 0.45
wspd_generate_data 12 0.9


## hitting set analysis
hs_analysis() {
    local d=$1
    local e=$2
    local outfile="out/${name}_hs_d${d}_e${e:2}.txt"
    local pathsfile="out/${name}_paths_d${d}_e${e:2}.txt"
    
    if ! test -f "$outfile"; then
        cargo run --release --bin hittingset_analysis -- -g $graph -o $outfile -p $pathsfile -l -i -v --skip_verification > analysis/${name}_hs_d${d}_e${e:2}.txt
    fi
}

hs_analysis 8 0.06
hs_analysis 9 0.15
hs_analysis 10 0.25
hs_analysis 11 0.45
hs_analysis 12 0.9

# run py script to generate images
python3 src/draw_plots.py
