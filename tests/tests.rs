/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
extern crate lib_dachshund;

use lib_dachshund::dachshund::candidate::Candidate;
use lib_dachshund::dachshund::error::{CLQError, CLQResult};
use lib_dachshund::dachshund::graph::{TypedGraphBuilder, Graph};
use lib_dachshund::dachshund::id_types::{GraphId, NodeId};
use lib_dachshund::dachshund::output::Output;
use lib_dachshund::dachshund::row::{CliqueRow, EdgeRow};
use lib_dachshund::dachshund::test_utils::{
    assert_nodes_have_ids, gen_single_clique, gen_test_transformer, gen_test_typespec,
    process_raw_vector,
};
use lib_dachshund::dachshund::transformer::Transformer;

#[cfg(test)]
#[test]
fn test_process_typespec() -> CLQResult<()> {
    let ts = vec![
        vec!["author".to_string(), "published_at".into(), "conference".into()],
        vec!["author".to_string(), "organized".into(), "conference".into()],
        vec!["author".to_string(), "published_at".into(), "journal".into()],
        vec!["author".to_string(), "attended".into(), "conference".into()],
    ];
    let target_types = vec!["conference".to_string(), "journal".into()];
    let core_type: String = "author".to_string();
    let target_type_ids = Transformer::process_typespec(ts, &core_type, target_types)?;
    assert_eq!(target_type_ids.require("conference")?.value(), 1);
    assert_eq!(target_type_ids.require("journal")?.value(), 2);
    assert_eq!(
        target_type_ids
            .require("conference")?
            .max_edge_count_with_core_node()
            .unwrap(),
        3
    );
    assert_eq!(
        target_type_ids
            .require("journal")?
            .max_edge_count_with_core_node()
            .unwrap(),
        1
    );
    Ok(())
}

#[test]
fn test_process_single_line() -> CLQResult<()> {
    let ts = gen_test_typespec();
    let transformer = gen_test_transformer(ts, "author".to_string())?;
    // graph_id source_id target_id target_type
    let raw: String = "0\t1\t2\tauthor\tpublished_at\tjournal".to_string();

    let row: EdgeRow = transformer
        .process_line(raw)?
        .as_edge_row()
        .ok_or_else(CLQError::err_none)?;
    assert_eq!(row.graph_id.value(), 0);
    assert_eq!(row.source_id, NodeId::from(1));
    assert_eq!(row.target_id, NodeId::from(2));
    let target_type_name: Option<String> =
        transformer.non_core_type_ids.type_name(&row.target_type_id);
    assert_eq!(target_type_name, Some("journal".to_owned()));
    Ok(())
}

#[test]
fn test_process_single_line_clique_row() -> CLQResult<()> {
    let ts = gen_test_typespec();
    let transformer = gen_test_transformer(ts, "author".to_string())?;
    // graph_id node_id node_type
    let raw: String = "0\t2\tjournal\t\t\t".to_string();
    let row: CliqueRow = transformer.process_line(raw)?.as_clique_row().unwrap();
    assert_eq!(row.graph_id.value(), 0);
    assert_eq!(row.node_id, NodeId::from(2));
    let target_type_name: Option<String> = transformer
        .non_core_type_ids
        .type_name(&row.target_type.unwrap());
    assert_eq!(target_type_name, Some("journal".to_owned()));
    let raw: String = "0\t1\tauthor\t\t\t".to_string();
    let row: CliqueRow = transformer.process_line(raw)?.as_clique_row().unwrap();
    assert_eq!(row.graph_id.value(), 0);
    assert_eq!(row.node_id, NodeId::from(1));
    assert_eq!(row.target_type, None);
    Ok(())
}

#[test]
fn test_process_single_row() -> CLQResult<()> {
    let ts = gen_test_typespec();
    let transformer = gen_test_transformer(ts, "author".to_string())?;
    // graph_id source_id target_id target_type
    let raw = "0\t1\t2\tauthor\tpublished_at\tconference".to_string();
    let graph_id: GraphId = 0.into();

    let row: EdgeRow = transformer.process_line(raw)?.as_edge_row().unwrap();
    let rows = vec![row];
    let mut buffer: Vec<u8> = Vec::new();
    let mut output = Output::string(&mut buffer);
    let graph: Graph =
        transformer.build_pruned_graph::<TypedGraphBuilder, Graph>(graph_id, &rows)?;
    let res: Candidate<Graph> = transformer
        .process_clique_rows::<TypedGraphBuilder, Graph>(
            &graph,
            Vec::new(),
            graph_id,
            true,
            &mut output,
        )?
        .ok_or_else(CLQError::err_none)?
        .top_candidate;
    assert_nodes_have_ids(&graph, &res.core_ids, vec![1], true);
    assert_nodes_have_ids(&graph, &res.non_core_ids, vec![2], false);
    Ok(())
}

#[test]
fn test_process_small_clique() -> CLQResult<()> {
    let ts = gen_test_typespec();
    // graph_id source_id target_id target_type
    let raw = vec![
        "0\t1\t3\tauthor\tpublished_at\tconference".to_string(),
        "0\t2\t3\tauthor\tpublished_at\tconference".into(),
        "0\t1\t4\tauthor\tpublished_at\tconference".into(),
        "0\t2\t4\tauthor\tpublished_at\tconference".into(),
    ];
    let graph_id: GraphId = 0.into();

    let transformer = gen_test_transformer(ts, "author".to_string())?;
    let rows = process_raw_vector(&transformer, raw)?;
    let mut buffer: Vec<u8> = Vec::new();
    let mut output = Output::string(&mut buffer);
    let graph: Graph =
        transformer.build_pruned_graph::<TypedGraphBuilder, Graph>(graph_id, &rows)?;
    let res: Candidate<Graph> = transformer
        .process_clique_rows::<TypedGraphBuilder, Graph>(
            &graph,
            Vec::new(),
            graph_id,
            true,
            &mut output,
        )?
        .ok_or_else(CLQError::err_none)?
        .top_candidate;
    assert_nodes_have_ids(&graph, &res.core_ids, vec![1, 2], true);
    assert_nodes_have_ids(&graph, &res.non_core_ids, vec![3, 4], false);
    Ok(())
}

#[test]
fn test_process_small_clique_with_non_clique_row() -> CLQResult<()> {
    let ts = gen_test_typespec();
    // graph_id source_id target_id target_type
    let raw = vec![
        "0\t1\t3\tauthor\tpublished_at\tconference".to_string(),
        "0\t2\t3\tauthor\tpublished_at\tconference".into(),
        "0\t1\t4\tauthor\tpublished_at\tconference".into(),
        "0\t2\t4\tauthor\tpublished_at\tconference".into(),
        // nonsensical
        "0\t2\t5\tconference\tpublished_at\tconference".into(),
    ];
    let graph_id: GraphId = 0.into();

    let transformer = gen_test_transformer(ts, "author".to_string())?;
    let rows = process_raw_vector(&transformer, raw)?;
    let mut buffer: Vec<u8> = Vec::new();
    let mut output = Output::string(&mut buffer);
    let graph: Graph =
        transformer.build_pruned_graph::<TypedGraphBuilder, Graph>(graph_id, &rows)?;
    let res: Candidate<Graph> = transformer
        .process_clique_rows::<TypedGraphBuilder, Graph>(
            &graph,
            Vec::new(),
            graph_id,
            true,
            &mut output,
        )?
        .ok_or_else(CLQError::err_none)?
        .top_candidate;
    assert_nodes_have_ids(&graph, &res.core_ids, vec![1, 2], true);
    assert_nodes_have_ids(&graph, &res.non_core_ids, vec![3, 4], false);
    Ok(())
}

#[test]
fn test_process_medium_clique() -> CLQResult<()> {
    let ts = gen_test_typespec();
    let non_core_types = ts.iter().map(|x| x[2].clone()).collect();
    let graph_id: GraphId = 0.into();
    let (core_ids, non_cores, clique_rows) = gen_single_clique(
        graph_id,
        10,
        vec![10, 10],
        non_core_types,
        "author".to_string(),
        vec!["published_at".to_string()],
    );
    assert_eq!(clique_rows.len(), 200);
    let transformer = gen_test_transformer(ts, "author".to_string())?;
    let rows = process_raw_vector(&transformer, clique_rows)?;
    let mut buffer: Vec<u8> = Vec::new();
    let mut output = Output::string(&mut buffer);
    let graph: Graph =
        transformer.build_pruned_graph::<TypedGraphBuilder, Graph>(graph_id, &rows)?;
    let res: Candidate<Graph> = transformer
        .process_clique_rows::<TypedGraphBuilder, Graph>(
            &graph,
            Vec::new(),
            graph_id,
            false,
            &mut output,
        )?
        .ok_or_else(CLQError::err_none)?
        .top_candidate;
    assert_nodes_have_ids(&graph, &res.core_ids, core_ids, true);
    let non_core_ids: Vec<NodeId> = non_cores.into_iter().map(|x| x.0).collect();
    assert_nodes_have_ids(&graph, &res.non_core_ids, non_core_ids, false);
    Ok(())
}

#[test]
fn test_process_medium_clique_with_insufficient_epochs() -> CLQResult<()> {
    let ts = gen_test_typespec();
    let non_core_types = ts.iter().map(|x| x[2].clone()).collect();
    let graph_id: GraphId = 0.into();
    let (_core_ids, _non_cores, clique_rows) = gen_single_clique(
        graph_id,
        10,
        vec![10, 10],
        non_core_types,
        "author".to_string(),
        vec!["published_at".to_string()],
    );
    assert_eq!(clique_rows.len(), 200);
    let transformer = Transformer::new(
        ts,
        20,
        1.0,
        Some(1.0),
        Some(1.0),
        20,
        10,
        3,
        true, // with 10 epochs
        0,    // min_degree = 0
        "author".to_string(),
        false,
    )?;
    let rows = process_raw_vector(&transformer, clique_rows)?;
    let mut buffer: Vec<u8> = Vec::new();
    let mut output = Output::string(&mut buffer);
    let graph: Graph =
        transformer.build_pruned_graph::<TypedGraphBuilder, Graph>(graph_id, &rows)?;
    let res: Candidate<Graph> = transformer
        .process_clique_rows::<TypedGraphBuilder, Graph>(
            &graph,
            Vec::new(),
            graph_id,
            false,
            &mut output,
        )?
        .ok_or_else(CLQError::err_none)?
        .top_candidate;
    // discovered clique size will be num_epochs + 1
    assert_eq!(res.core_ids.len() + res.non_core_ids.len(), 11);
    Ok(())
}

#[test]
fn test_process_small_clique_with_two_kinds_of_rows() -> CLQResult<()> {
    let typespec = vec![
        vec!["author".to_string(), "published_at".into(), "conference".into()],
        vec!["author".to_string(), "attended".into(), "conference".into()],
    ];
    let raw = vec![
        "0\t1\t3\tauthor\tpublished_at\tconference".to_string(),
        "0\t2\t3\tauthor\tpublished_at\tconference".into(),
        "0\t1\t3\tauthor\tattended\tconference".into(),
        "0\t2\t3\tauthor\tattended\tconference".into(),
    ];
    let graph_id: GraphId = 0.into();

    let transformer = gen_test_transformer(typespec, "author".to_string())?;
    let rows = process_raw_vector(&transformer, raw)?;
    let mut buffer: Vec<u8> = Vec::new();
    let mut output = Output::string(&mut buffer);
    let graph: Graph =
        transformer.build_pruned_graph::<TypedGraphBuilder, Graph>(graph_id, &rows)?;
    let res: Candidate<Graph> = transformer
        .process_clique_rows::<TypedGraphBuilder, Graph>(
            &graph,
            Vec::new(),
            graph_id,
            true,
            &mut output,
        )?
        .ok_or_else(CLQError::err_none)?
        .top_candidate;
    assert_nodes_have_ids(&graph, &res.core_ids, vec![1, 2], true);
    assert_nodes_have_ids(&graph, &res.non_core_ids, vec![3], false);
    Ok(())
}

#[test]
fn test_process_another_small_clique_with_two_kinds_of_rows() -> CLQResult<()> {
    let typespec = vec![
        vec![
            "author".to_string(),
            "published".into(),
            "article".into(),
        ],
        vec![
            "author".to_string(),
            "cited".into(),
            "article".into(),
        ],
    ];
    let raw = vec![
        "0\t1\t5\tauthor\tpublished\tarticle".to_string(),
        "0\t0\t5\tauthor\tpublished\tarticle".into(),
        "0\t2\t5\tauthor\tpublished\tarticle".into(),
        "0\t3\t5\tauthor\tpublished\tarticle".into(),
        "0\t2\t5\tauthor\tcited\tarticle".into(),
        "0\t4\t5\tauthor\tpublished\tarticle".into(),
        "0\t3\t5\tauthor\tcited\tarticle".into(),
    ];
    let graph_id: GraphId = 0.into();

    let transformer = gen_test_transformer(typespec, "author".to_string())?;
    let rows = process_raw_vector(&transformer, raw)?;
    let mut buffer: Vec<u8> = Vec::new();
    let mut output = Output::string(&mut buffer);
    let graph: Graph =
        transformer.build_pruned_graph::<TypedGraphBuilder, Graph>(graph_id, &rows)?;
    let res: Candidate<Graph> = transformer
        .process_clique_rows::<TypedGraphBuilder, Graph>(
            &graph,
            Vec::new(),
            graph_id,
            true,
            &mut output,
        )?
        .ok_or_else(CLQError::err_none)?
        .top_candidate;
    assert_nodes_have_ids(&graph, &res.core_ids, vec![2, 3], true);
    assert_nodes_have_ids(&graph, &res.non_core_ids, vec![5], false);
    Ok(())
}
