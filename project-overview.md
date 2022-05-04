# Project structure

## Data Structures

### Graph
To reduce memory usage, the graph structure is split into parts, each only containing information that is required for a specific algorithm.
This split is implemented using interfaces (`Traits` in Rust) and each algorithm is implemented based on these interfaces.
One could implement a graph class that implements all interfaces in one class instead, but using this splitting strategy allows running some of the code (e.g. the visualization part) with the german road network on a 16GB machine.

Basic graph traits are defined in [graph.rs](src/graph.rs).
Additionally, combinations of these traits define a few graph types:
*	CHGraph: implements methods to run a CH-Dijkstra search.
*	HSGraph: implements methods to find node or edge coverings in G_M as defined in the thesis text.
*	GeoGraph: implements methods to draw the graph on a map using latitude and longitude.

Relevant files are:
* [graph/chgraph.rs](src/graph/chgraph.rs) - struct that implements the CHGraph and GeoGraph traits.
* [graph/hsgraph.rs](src/graph/hsgraph.rs) - struct that implements the HSGraph trait.
* [graph/fmigraph.rs](src/graph/fmigraph.rs) - struct that mirrors the .fmi graph file format.

To load a graph from a `*.fmi` file, first read the file into a FMIGraph. The code parsing `*.fmi` files is found in [file_handling.rs](src/file_handling.rs). Then convert the FMI graph into a CH or HS graph as required. Conversion code is found in the respective `graph/*.rs` files.

### QuadTree
The quadtree is implemented in [quadtree.rs](src/quadtree.rs).
This implementation contains a few features that are used for visualization:
all cells are named with a "[abcd]+" string, where the topleft child is called 'a', topright 'b', bottomleft 'c', bottomright 'd'. Starting from the end of the name, one can follow the string to find the corresponding cell (e.g. 'abc' is cell bottomleft->topright->topleft).
These names are used for visualization or for identifying cells where specific checks fail.

Additionally, the projection into a unit square is implemented in quadtree.rs as well.
During QuadTree construction, nodes are first projected onto the plane using the mercator projection, then scaled (using a stateful scaler to allow for inverse scaling). Inverse scaling is used to draw cells onto the map (corners of cells are not nodes in the graph => inverse scaling is required).

## Algorithms
### Dijkstra's algorithm

The CH-Search algorithm is implemented in [dijkstra.rs](src/dijkstra.rs) using the optimized dijkstra variant from https://github.com/Lesstat/dijkstra-performance-study/.

### WSPD
In [wspd.rs](src/wspd.rs) the algWSPD is implemented as described in Geometric Approximation Algorithms (Har-Peled, Sariel. Geometric approximation algorithms. American Mathematical Soc., 2011).
The result is stored as array of references into the quadtree data structure.

### Hitting Set
The hitting set algorithm is implemented in [hittingset.rs](src/hittingset.rs) as described in the thesis. Additionally it is possible to stop the algorithm after a specified number of iterations. The instance based lower bound calculation is implemented as well. 
For analysis, detailed information on each iteration can be accessed.


## Utility Files
There are a few files providing additional utility functions:
* [random_pairs.rs](src/random_pairs.rs): functions to sample random point pairs
* [vis.rs](src/vis.rs) and files in the vis subdir: Interface and classes that provide map drawing utilities based on the builder pattern
* [paths.rs](src/paths.rs): defines paths and related functions
* [lib.rs](src/lib.rs): general file reading utilities: setting correct parameters for file reader libraries, caching objects to binary files, buffered writing
