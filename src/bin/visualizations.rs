use std::iter::repeat;

use garp::{
    dijkstra::Dijkstra,
    graph::{BaseGraph, CHGraph},
    load_ch_graph, load_hittingset, load_paths,
    quadtree::{QuadTree, TreeNode},
    vis::{Color, GeoJsonBuilder, MapBuilder, VisBuilder},
    wspd::{Tree, WSPD},
};
use clap::{App, Arg};

// #[derive(Parser)]
// #[clap(author, version, about, long_about = None)]
// #[clap(subcommandsRequired)]
struct Args<'a> {
    /// if the output should be an image instead of geojson
    // #[clap(short, long)]
    image: bool,
    /// .fmi graph file
    // #[clap(short, long)]
    graph_file: &'a str, //String,
    /// file to store paths
    // #[clap(short, long)]
    out_file: &'a str, //String,
    // file containing paths
    paths_file: Option<&'a str>,
    // file containing hitting set
    hs_file: Option<&'a str>,
    /// draw full quad tree
    quad_tree: bool,
    /// draw all points in a cell up to n points
    points_per_cell: Option<usize>,
    /// draw all pairs of a given cell
    cluster_of_cell: Option<&'a str>,
    /// depth of the tree
    tree_depth: Option<usize>,
    /// wspd epsilon
    epsilon: f64,
    /// cell pair to draw with 3 points and paths each
    cell_pair: Option<(&'a str, &'a str)>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("visualization utiility")
        .arg(
            Arg::with_name("static image")
                .long("image")
                .short("i")
                .required(false),
        )
        .arg(
            Arg::with_name("graph file")
                .short("g")
                .help(".fmi graph file")
                .takes_value(true)
                .value_name("*.fmi")
                .required(true),
        )
        .arg(
            Arg::with_name("out file")
                .short("o")
                .help("output file")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("paths")
                .long("paths")
                .short("p")
                .takes_value(true)
                .help("draws paths listed in the input file"),
        )
        .arg(
            Arg::with_name("hs file")
                .long("hsfile")
                .short("h")
                .takes_value(true)
                .help("draws hittingset given in the input file"),
        )
        .arg(
            Arg::with_name("full tree")
                .long("tree")
                .help("draws all cells of the tree")
                .requires("tree depth"),
        )
        .arg(
            Arg::with_name("tree depth")
                .long("depth")
                .short("d")
                .takes_value(true)
                .help("depth of the tree"),
        )
        .arg(
            Arg::with_name("points per cell")
                .long("points-per-cell")
                .help("draws up to n points for each tree cell")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("cluster of cell")
                .long("cluster-of-cell")
                .help("draws all cluster pairs of given cell and their path")
                .takes_value(true)
                .requires("tree depth")
                .required(false),
        )
        .arg(
            Arg::with_name("epsilon")
                .long("epsilon")
                .takes_value(true)
                .default_value("0.5"),
        )
        .arg(
            Arg::with_name("cell pair")
                .takes_value(true)
                .min_values(2)
                .max_values(2)
                .long("cell-pair"),
        )
        .get_matches();
    // TODO: epsilon for wspd

    let args = Args {
        graph_file: matches.value_of("graph file").unwrap(),
        image: matches.is_present("static image"),
        out_file: matches.value_of("out file").unwrap(),
        paths_file: matches.value_of("paths"),
        hs_file: matches.value_of("hs file"),
        tree_depth: matches.value_of("tree depth").and_then(|d| d.parse().ok()),
        points_per_cell: matches
            .value_of("points per cell")
            .and_then(|d| d.parse().ok()),
        cluster_of_cell: matches.value_of("cluster of cell"),
        quad_tree: matches.is_present("full tree"),
        epsilon: matches.value_of("epsilon").unwrap().parse().unwrap(),
        cell_pair: matches
            .values_of("cell pair")
            .and_then(|mut v| Some((v.next().unwrap(), v.next().unwrap()))),
    };

    let graph = load_ch_graph(args.graph_file)?;

    let mut dijkstra = Dijkstra::new(&graph);

    let mut map_builder = MapBuilder::germany(&graph)?;
    let mut geo_builder = GeoJsonBuilder::new(&graph);

    let builder: &mut dyn VisBuilder = if args.image {
        &mut map_builder
    } else {
        &mut geo_builder
    };

    if let Some(paths_file) = args.paths_file {
        let paths = load_paths(paths_file)?;

        // draw paths
        for path in paths {
            let full_path = graph.unpack_ch_edges(&path);
            builder.path(&full_path);
        }
    }

    if let Some(hs_file) = args.hs_file {
        let hittingset = load_hittingset(hs_file);
        // draw nodes
        for h in hittingset {
            builder.point(h);
        }
    }

    let quadtree = if let Some(depth) = args.tree_depth {
        let tree_data = graph
            .iter_nodes()
            .map(|n| {
                let tn: &dyn TreeNode = n;
                tn
            })
            .collect();
        let quadtree = QuadTree::new(tree_data, depth);
        Some(quadtree)
    } else {
        None
    };

    if args.quad_tree {
        recursive_tree_draw(
            quadtree.as_ref().expect("tree depth required"),
            builder,
            args.points_per_cell.unwrap_or(0),
        );
    }

    if let Some(cell) = args.cluster_of_cell {
        let qt = quadtree.as_ref().expect("tree depth required");
        // calculate cluster
        let wspd = WSPD::new(qt, args.epsilon);

        // cell we are looking for
        let cell = qt.get_by_id(cell.to_string());

        draw_cell(cell, builder, args.points_per_cell.unwrap_or(0));

        for (u, v) in wspd.iter() {
            if u.id() == cell.id() || v.id() == cell.id() {
                // draw the one != cell and a shortest path
                if u.id() != cell.id() {
                    draw_cell(&u, builder, args.points_per_cell.unwrap_or(0));
                } else {
                    draw_cell(&v, builder, args.points_per_cell.unwrap_or(0));
                }
                // shortest path
                let path = dijkstra.ch_search(u.rep().id(), v.rep().id());
                if let Some((_, p)) = path {
                    let full_path = graph.unpack_ch_edges(&p);
                    builder.path(&full_path);
                }
            }
        }
    }

    if let Some((u, v)) = args.cell_pair {
        let qt = quadtree.as_ref().expect("tree depth required");

        let u = qt.get_by_id(u.to_string());
        let v = qt.get_by_id(v.to_string());

        draw_cell(u, builder, args.points_per_cell.unwrap_or(0));
        draw_cell(v, builder, args.points_per_cell.unwrap_or(0));

        let u_points: Vec<&&dyn TreeNode> = u.points().collect();
        let v_points: Vec<&&dyn TreeNode> = v.points().collect();
        let u_samples = &u_points[0..3.min(u_points.len())];
        let v_samples = &v_points[0..3.min(v_points.len())];

        println!("ulen: {}, vlen: {}", u_samples.len(), v_samples.len());

        let mut count = 0;
        for (u_node, v_node) in u_samples
            .into_iter()
            .flat_map(|u| repeat(u).zip(v_samples.into_iter()))
        {
            count += 1;
            if let Some((_dist, path)) = dijkstra.ch_search(u_node.id(), v_node.id()) {
                builder.path(&graph.unpack_ch_edges(&path));
            } else {
                println!("no path: {:?}, {:?}", u_node.id(), v_node.id());
                println!(
                    "{:?}: lat={}, lon={}",
                    u_node.id(),
                    u_node.lat(),
                    u_node.lon()
                );
                println!(
                    "{:?}: lat={}, lon={}",
                    v_node.id(),
                    v_node.lat(),
                    v_node.lon()
                );
                if let Some((_dist, path)) = dijkstra.ch_search(v_node.id(), u_node.id()) {
                    builder.path(&graph.unpack_ch_edges(&path));
                    println!("rev possible");
                } else {
                    println!("rev not possible");
                }
            }
        }
        println!("count: {}", count);
    }

    builder.save(args.out_file);

    Ok(())
}

fn draw_cell(tree: &QuadTree, builder: &mut dyn VisBuilder, points_per_cell: usize) {
    let cell_color = Color::random();
    // draw cell edges
    for (p1, p2) in tree.cell_edges() {
        builder.line_with_color(p1, p2, cell_color);
    }

    // draw points
    if points_per_cell == 1 {
        builder.point_with_color(tree.rep().id(), cell_color);
    } else {
        for (count, &p) in tree.points().enumerate() {
            if count >= points_per_cell {
                break;
            }
            builder.point_with_color(p.id(), cell_color);
        }
    }
}

fn recursive_tree_draw(tree: &QuadTree, builder: &mut dyn VisBuilder, points_per_cell: usize) {
    draw_cell(tree, builder, points_per_cell);

    for c in tree.children() {
        recursive_tree_draw(c, builder, points_per_cell);
    }
}
