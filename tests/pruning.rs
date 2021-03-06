/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
extern crate lib_dachshund;
use lib_dachshund::dachshund::candidate::Candidate;
use lib_dachshund::dachshund::error::{CLQError, CLQResult};
use lib_dachshund::dachshund::graph::{TypedGraphBuilder, Graph, GraphBuilder};
use lib_dachshund::dachshund::id_types::{GraphId, NodeId};
use lib_dachshund::dachshund::output::Output;
use lib_dachshund::dachshund::test_utils::{
    assert_nodes_have_ids, gen_test_transformer, process_raw_vector,
};
use lib_dachshund::dachshund::transformer::Transformer;
use std::collections::HashSet;

pub fn gen_test_typespec() -> Vec<Vec<String>> {
    return vec![
        vec!["author".into(), "published_at".into(), "conference".into()],
        vec!["author".into(), "published_at".into(), "journal".into()],
        vec!["author".into(), "reviewed_for".into(), "conference".into()],
        vec!["author".into(), "reviewed_for".into(), "journal".into()],
        vec!["author".into(), "administered".into(), "conference".into()],
        vec!["author".into(), "administered".into(), "journal".into()],
    ];
}
fn simple_test(raw: Vec<String>, min_degree: usize, expected_len: usize) -> CLQResult<()> {
    let typespec = vec![
        vec!["author".into(), "published_at".into(), "conference".into()],
        vec!["author".into(), "reviewed_for".into(), "conference".into()],
    ];
    let graph_id: GraphId = 0.into();

    let transformer = gen_test_transformer(typespec, "author".to_string())?;
    let rows = process_raw_vector(&transformer, raw)?;

    let mut graph: Graph =
        transformer.build_pruned_graph::<TypedGraphBuilder, Graph>(graph_id, &rows)?;
    let exclude_nodes: HashSet<NodeId> =
        TypedGraphBuilder::trim_edges(&mut graph.nodes, &min_degree);
    assert_eq!(exclude_nodes.len(), expected_len);
    Ok(())
}

#[cfg(test)]
#[test]
fn test_no_exclude_nodes() -> CLQResult<()> {
    let raw = vec![
        "0\t1\t3\tauthor\tpublished_at\tconference".into(),
        "0\t2\t3\tauthor\tpublished_at\tconference".into(),
        "0\t1\t3\tauthor\treviewed_for\tconference".into(),
        "0\t2\t3\tauthor\treviewed_for\tconference".into(),
    ];
    simple_test(raw, 1, 0)
}

#[test]
fn test_all_exclude_nodes() -> CLQResult<()> {
    let raw = vec![
        "0\t1\t3\tauthor\tpublished_at\tconference".into(),
        "0\t2\t3\tauthor\tpublished_at\tconference".into(),
        "0\t1\t3\tauthor\treviewed_for\tconference".into(),
        "0\t2\t3\tauthor\treviewed_for\tconference".into(),
    ];
    simple_test(raw, 3, 3)
}

#[test]
fn test_partial_exclude_nodes() -> CLQResult<()> {
    let raw = vec![
        "0\t1\t3\tauthor\tpublished_at\tconference".into(),
        "0\t2\t3\tauthor\tpublished_at\tconference".into(),
        "0\t1\t3\tauthor\treviewed_for\tconference".into(),
    ];
    simple_test(raw, 2, 1)
}

#[test]
fn test_prune_small_clique() -> CLQResult<()> {
    // graph_id source_id target_id target_type
    let ts: Vec<Vec<String>> = vec![vec!["author".into(), "published_at".into(), "conference".into()]];
    let raw = vec![
        "0\t1\t3\tauthor\tpublished_at\tconference".into(),
        "0\t2\t3\tauthor\tpublished_at\tconference".into(),
        "0\t1\t4\tauthor\tpublished_at\tconference".into(),
        "0\t2\t4\tauthor\tpublished_at\tconference".into(),
        "0\t2\t5\tauthor\tpublished_at\tconference".into(),
    ];
    let graph_id: GraphId = 0.into();

    let transformer = gen_test_transformer(ts, "author".to_string())?;
    let rows = process_raw_vector(&transformer, raw)?;
    let mut graph: Graph =
        transformer.build_pruned_graph::<TypedGraphBuilder, Graph>(graph_id, &rows)?;
    assert_eq!(graph.nodes.len(), 5);
    graph = TypedGraphBuilder::prune(graph, &rows, 2)?;
    assert_eq!(graph.nodes.len(), 4);
    let res: Candidate<Graph> = transformer
        .process_graph::<Graph>(&graph, Vec::new(), graph_id, true)?
        .top_candidate;
    assert_nodes_have_ids(&graph, &res.core_ids, vec![1, 2], true);
    assert_nodes_have_ids(&graph, &res.non_core_ids, vec![3, 4], false);
    Ok(())
}

#[test]
// proves that pruning indeed shortens the amount of cycles it takes to get results
fn test_full_prune_small_clique() -> CLQResult<()> {
    // graph_id source_id target_id target_type
    let raw = vec![
        "0\t1\t3\tauthor\tpublished_at\tconference".into(),
        "0\t2\t3\tauthor\tpublished_at\tconference".into(),
        "0\t1\t3\tauthor\treviewed_for\tconference".into(),
        "0\t2\t3\tauthor\treviewed_for\tconference".into(),
        "0\t1\t3\tauthor\tadministered\tconference".into(),
        "0\t2\t3\tauthor\tadministered\tconference".into(),
        "0\t1\t4\tauthor\tpublished_at\tconference".into(),
        "0\t2\t4\tauthor\tpublished_at\tconference".into(),
        "0\t2\t5\tauthor\tpublished_at\tconference".into(),
        "0\t6\t7\tauthor\tpublished_at\tconference".into(),
        "0\t8\t9\tauthor\tpublished_at\tconference".into(),
        "0\t10\t11\tauthor\tpublished_at\tconference".into(),
    ];
    let graph_id: GraphId = 0.into();

    let ts = gen_test_typespec();
    // with pruning at degree < 3
    let transformer_prune = Transformer::new(
        ts.clone(),
        20,
        1.0,
        Some(1.0),
        Some(1.0),
        20,
        10000,
        3,
        false,
        3,
        "author".into(),
        false,
    )?;
    let rows_prune = process_raw_vector(&transformer_prune, raw.clone())?;

    let graph: Graph = transformer_prune
        .build_pruned_graph::<TypedGraphBuilder, Graph>(graph_id, &rows_prune)?;
    let mut text_prune: Vec<u8> = Vec::new();
    let mut output_prune = Output::string(&mut text_prune);
    let result_prune = transformer_prune
        .process_clique_rows::<TypedGraphBuilder, Graph>(
            &graph,
            Vec::new(),
            graph_id,
            false,
            &mut output_prune,
        )?
        .ok_or_else(CLQError::err_none)?;
    let candidate_prune = result_prune.top_candidate;
    assert_nodes_have_ids(&graph, &candidate_prune.core_ids, vec![1, 2], true);
    assert_nodes_have_ids(&graph, &candidate_prune.non_core_ids, vec![3], false);

    // without any pruning
    let transformer = Transformer::new(
        ts,
        20,
        1.0,
        Some(1.0),
        Some(1.0),
        20,
        10000,
        3,
        false,
        0,
        "author".into(),
        false,
    )?;
    let rows = process_raw_vector(&transformer, raw)?;

    let graph: Graph =
        transformer.build_pruned_graph::<TypedGraphBuilder, Graph>(graph_id, &rows)?;
    let mut text: Vec<u8> = Vec::new();
    let mut output = Output::string(&mut text);
    let result = transformer
        .process_clique_rows::<TypedGraphBuilder, Graph>(
            &graph,
            Vec::new(),
            graph_id,
            false,
            &mut output,
        )?
        .ok_or_else(CLQError::err_none)?;
    let candidate = result.top_candidate;
    assert_nodes_have_ids(&graph, &candidate.core_ids, vec![1, 2], true);
    assert_nodes_have_ids(&graph, &candidate.non_core_ids, vec![3], false);
    println!("Num steps prune: {}", result_prune.num_steps);
    println!("Num steps: {}", result.num_steps);
    assert!(result_prune.num_steps < result.num_steps);
    Ok(())
}
