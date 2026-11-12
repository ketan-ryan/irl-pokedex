use petgraph::{
    Direction::{Incoming, Outgoing},
    algo::toposort,
    graph::{DiGraph, NodeIndex},
};
use std::collections::{HashMap, HashSet};

use crate::enums::PokemonInfo;

pub fn build_evo_graph(start: &str, get: impl Fn(&str) -> PokemonInfo) -> DiGraph<String, ()> {
    let mut g = DiGraph::new();
    let mut nodes: HashMap<String, NodeIndex> = HashMap::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut stack = vec![start.to_lowercase()];

    let idx = |g: &mut DiGraph<String, ()>, nodes: &mut HashMap<String, NodeIndex>, name: &str| {
        *nodes
            .entry(name.to_string())
            .or_insert_with(|| g.add_node(name.to_string()))
    };

    // lowercase first, then drop megas
    let neighbors = |l: Option<Vec<String>>| -> Vec<String> {
        l.unwrap_or_default()
            .into_iter()
            .map(|n| n.to_lowercase())
            .filter(|n| !n.contains("mega") && get(n).base.unwrap_or(true))
            .collect()
    };

    while let Some(name) = stack.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        let info = get(&name);
        let me = idx(&mut g, &mut nodes, &name);

        for from in neighbors(info.evolves_from) {
            let f = idx(&mut g, &mut nodes, &from);
            g.update_edge(f, me, ());
            stack.push(from);
        }
        for into in neighbors(info.evolves_into) {
            let t = idx(&mut g, &mut nodes, &into);
            g.update_edge(me, t, ());
            stack.push(into);
        }
    }
    g
}

/// Stage (column) of each node, indexed by `NodeIndex::index()`.
pub fn stages(g: &DiGraph<String, ()>) -> Vec<usize> {
    // If bad data creates a cycle, fall back to arbitrary order instead of panicking
    let order = toposort(g, None).unwrap_or_else(|_| g.node_indices().collect());
    let mut stage = vec![0usize; g.node_count()];
    for n in order {
        for m in g.neighbors_directed(n, Outgoing) {
            stage[m.index()] = stage[m.index()].max(stage[n.index()] + 1);
        }
    }
    stage
}

/// Nodes grouped by stage. Within a stage, order by the row of the parents
/// so branches don't cross.
pub fn columns(g: &DiGraph<String, ()>) -> Vec<Vec<NodeIndex>> {
    let stage = stages(g);
    let n_cols = stage.iter().max().map_or(0, |m| m + 1);
    let mut cols = vec![Vec::new(); n_cols];
    let mut row_of: HashMap<NodeIndex, usize> = HashMap::new();

    for (s, col) in cols.iter_mut().enumerate() {
        let mut nodes: Vec<NodeIndex> =
            g.node_indices().filter(|i| stage[i.index()] == s).collect();
        nodes.sort_by_key(|&n| {
            let parent_row = g
                .neighbors_directed(n, Incoming)
                .filter_map(|p| row_of.get(&p).copied())
                .min()
                .unwrap_or(0);
            (parent_row, n.index())
        });
        for (r, &n) in nodes.iter().enumerate() {
            row_of.insert(n, r);
        }
        *col = nodes;
    }
    cols
}

/// Every root-to-leaf path (used for the scrolling layout).
pub fn paths(g: &DiGraph<String, ()>) -> Vec<Vec<NodeIndex>> {
    fn walk(g: &DiGraph<String, ()>, path: Vec<NodeIndex>, out: &mut Vec<Vec<NodeIndex>>) {
        let last = *path.last().unwrap();
        let mut next: Vec<_> = g
            .neighbors_directed(last, Outgoing)
            .filter(|n| !path.contains(n)) // guard against cycles in bad data
            .collect();
        next.sort();
        if next.is_empty() {
            out.push(path);
            return;
        }
        for n in next {
            let mut p = path.clone();
            p.push(n);
            walk(g, p, out);
        }
    }

    let mut out = Vec::new();
    for root in g
        .node_indices()
        .filter(|&i| g.neighbors_directed(i, Incoming).next().is_none())
    {
        walk(g, vec![root], &mut out);
    }
    out
}

pub fn locate(
    graph: &DiGraph<String, ()>,
    cols: &[Vec<NodeIndex>],
    scroll_mode: bool,
    name: &str,
) -> Option<(usize, usize)> {
    let node = graph.node_indices().find(|&i| graph[i] == name)?;

    if scroll_mode {
        // one row per path; take the first path containing the node
        paths(graph)
            .iter()
            .enumerate()
            .find_map(|(row, p)| p.iter().position(|&n| n == node).map(|col| (row, col)))
    } else {
        cols.iter()
            .enumerate()
            .find_map(|(col, nodes)| nodes.iter().position(|&n| n == node).map(|row| (row, col)))
    }
}

pub fn node_at(
    graph: &DiGraph<String, ()>,
    cols: &[Vec<NodeIndex>],
    scroll_mode: bool,
    row: usize,
    col: usize,
) -> Option<NodeIndex> {
    if scroll_mode {
        paths(graph).get(row)?.get(col).copied()
    } else {
        cols.get(col)?.get(row).copied()
    }
}
