use super::Result;
use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
use writfold_domain::{GraphCommitId, GraphId, RowLimit};
use writfold_runtime::omnigraph::{
    NativeCall, NativeServer, Parameter, RawResponse, Scalar, StoredQuery, TransportOutcome,
};
use writfold_wire::{
    Limits,
    omnigraph::{
        Cell, Column, ConditionalChange, MutationBinding, Projection, QueryRows, QueryTarget,
        ScalarKind, decode_change, decode_complete_history, decode_precondition_refusal,
        decode_query, decode_retention,
    },
};

const COLUMNS: &[Column<'_>] = &[
    Column {
        name: "key",
        kind: ScalarKind::Text,
        nullable: false,
    },
    Column {
        name: "value",
        kind: ScalarKind::Text,
        nullable: false,
    },
];
fn limits() -> Result<Limits> {
    Ok(Limits::new(1 << 20, 32, 1024, 8192)?)
}
fn call(cancelled: &AtomicBool) -> NativeCall<'_> {
    NativeCall {
        deadline: Instant::now() + Duration::from_secs(30),
        cancelled,
    }
}
fn inventory(graph: &GraphId) -> StoredQuery<'_> {
    StoredQuery {
        actor: "act-reader",
        graph,
        name: "inventory",
        params: &[],
    }
}

pub fn exercise(
    server: &mut NativeServer<'_>,
    work: &Path,
    graph: &GraphId,
    cancelled: &AtomicBool,
) -> Result {
    let initial = ready(server, work, graph, cancelled)?;
    if initial.rows().len() != 0 {
        return Err("smoke requires an empty disposable graph".into());
    }
    let seeded = mutate(
        server,
        work,
        graph,
        cancelled,
        "insert_cell",
        initial.commit(),
        "native-before",
    )?;
    let before = pinned(
        server,
        work,
        graph,
        cancelled,
        "pinned-before",
        seeded.commit(),
    )?;
    one_row(&before, "native-before")?;
    retain(server, work, graph, cancelled, seeded.commit())?;
    let changed = mutate(
        server,
        work,
        graph,
        cancelled,
        "change",
        seeded.commit(),
        "native-after",
    )?;
    stale(
        server,
        work,
        graph,
        cancelled,
        seeded.commit(),
        changed.commit(),
    )?;
    verify_pins(
        server,
        work,
        graph,
        cancelled,
        initial.commit(),
        seeded.commit(),
        changed.commit(),
    )?;
    history(
        server,
        work,
        graph,
        cancelled,
        initial.commit(),
        seeded.commit(),
        changed.commit(),
    )?;
    Ok(())
}

fn retain(
    server: &mut NativeServer<'_>,
    work: &Path,
    graph: &GraphId,
    cancelled: &AtomicBool,
    pin: &GraphCommitId,
) -> Result {
    let response = observe(
        work,
        "retain",
        server.retain_branch(
            "act-writer",
            graph,
            "retained-native-smoke",
            pin,
            call(cancelled),
        ),
    )?;
    decode_retention(
        response.status,
        &response.body,
        limits()?,
        "retained-native-smoke",
        "act-writer",
        pin,
    )?;
    Ok(())
}

fn history(
    server: &mut NativeServer<'_>,
    work: &Path,
    graph: &GraphId,
    cancelled: &AtomicBool,
    original: &GraphCommitId,
    before: &GraphCommitId,
    after: &GraphCommitId,
) -> Result {
    let response = observe(
        work,
        "history",
        server.history("act-reader", graph, "main", 100, None, call(cancelled)),
    )?;
    let commits = decode_complete_history(
        response.status,
        &response.body,
        limits()?,
        after,
        RowLimit::new(100)?,
    )?;
    if !commits.contains(before) || !commits.contains(original) {
        return Err("complete native history lost a requested pin".into());
    }
    Ok(())
}

fn mutate(
    server: &mut NativeServer<'_>,
    work: &Path,
    graph: &GraphId,
    cancelled: &AtomicBool,
    name: &str,
    predecessor: &GraphCommitId,
    value: &str,
) -> Result<ConditionalChange> {
    let params = [
        Parameter {
            name: "key",
            value: Scalar::String("a"),
        },
        Parameter {
            name: "value",
            value: Scalar::String(value),
        },
    ];
    let response = observe(
        work,
        name,
        server.conditional_query(
            StoredQuery {
                actor: "act-writer",
                graph,
                name,
                params: &params,
            },
            "main",
            predecessor,
            call(cancelled),
        ),
    )?;
    let result = decode_change(
        response.status,
        &response.body,
        limits()?,
        MutationBinding {
            query: name,
            actor: "act-writer",
            predecessor,
        },
    )?;
    if (result.affected_nodes(), result.affected_edges()) != (1, 0) {
        return Err("mutation did not affect exactly the requested node".into());
    }
    Ok(result)
}

fn stale(
    server: &mut NativeServer<'_>,
    work: &Path,
    graph: &GraphId,
    cancelled: &AtomicBool,
    predecessor: &GraphCommitId,
    current: &GraphCommitId,
) -> Result {
    let params = [
        Parameter {
            name: "key",
            value: Scalar::String("a"),
        },
        Parameter {
            name: "value",
            value: Scalar::String("native-after"),
        },
    ];
    let response = observe(
        work,
        "stale",
        server.conditional_query(
            StoredQuery {
                actor: "act-writer",
                graph,
                name: "change",
                params: &params,
            },
            "main",
            predecessor,
            call(cancelled),
        ),
    )?;
    let refused =
        decode_precondition_refusal(&response.body, limits()?, response.status, predecessor)?;
    if refused.actual() != Some(current) {
        return Err("stale refusal named a different head".into());
    }
    Ok(())
}

fn verify_pins(
    server: &mut NativeServer<'_>,
    work: &Path,
    graph: &GraphId,
    cancelled: &AtomicBool,
    original: &GraphCommitId,
    before: &GraphCommitId,
    after: &GraphCommitId,
) -> Result {
    one_row(
        &pinned(server, work, graph, cancelled, "current", after)?,
        "native-after",
    )?;
    let retained = observe(
        work,
        "retained-read",
        server.branch_query(inventory(graph), "retained-native-smoke", call(cancelled)),
    )?;
    let retained = read_rows(&retained, QueryTarget::Branch("retained-native-smoke"))?;
    if retained.commit() != before {
        return Err("retained branch moved".into());
    }
    one_row(&retained, "native-before")?;
    if pinned(server, work, graph, cancelled, "original-empty", original)?
        .rows()
        .len()
        != 0
    {
        return Err("original empty snapshot changed".into());
    }
    Ok(())
}

fn pinned(
    server: &mut NativeServer<'_>,
    work: &Path,
    graph: &GraphId,
    cancelled: &AtomicBool,
    name: &str,
    pin: &GraphCommitId,
) -> Result<QueryRows> {
    let response = observe(
        work,
        name,
        server.pinned_query(inventory(graph), pin, call(cancelled)),
    )?;
    read_rows(&response, QueryTarget::Snapshot(pin))
}

fn ready(
    server: &mut NativeServer<'_>,
    work: &Path,
    graph: &GraphId,
    cancelled: &AtomicBool,
) -> Result<QueryRows> {
    let deadline = Instant::now() + Duration::from_secs(45);
    for attempt in 0..1000 {
        let response = observe(
            work,
            &format!("readiness-{attempt}"),
            server.branch_query(
                inventory(graph),
                "main",
                NativeCall {
                    deadline,
                    cancelled,
                },
            ),
        )?;
        if response.status == 200 {
            return read_rows(&response, QueryTarget::Branch("main"));
        }
        if response.status != 503 {
            return Err("native readiness returned a non-transient refusal".into());
        }
        if Instant::now() >= deadline {
            return Err("native readiness deadline".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err("native readiness attempt limit".into())
}

fn read_rows(response: &RawResponse, target: QueryTarget<'_>) -> Result<QueryRows> {
    Ok(decode_query(
        response.status,
        &response.body,
        limits()?,
        RowLimit::new(16)?,
        "inventory",
        target,
        Projection::new(COLUMNS)?,
    )?)
}
fn one_row(rows: &QueryRows, expected: &str) -> Result {
    if rows.rows().len() != 1
        || !matches!(rows.rows().next(), Some([Cell::Text(key), Cell::Text(value)]) if key == "a" && value == expected)
    {
        return Err("complete native row differs from the intended bytes".into());
    }
    Ok(())
}
fn observe(work: &Path, name: &str, outcome: TransportOutcome) -> Result<RawResponse> {
    match outcome {
        TransportOutcome::Observed(response) => {
            fs::write(work.join(format!("{name}.headers")), &response.headers)?;
            fs::write(work.join(format!("{name}.body")), &response.body)?;
            Ok(response)
        }
        TransportOutcome::Unissued(error) => {
            fs::write(work.join(format!("{name}.unissued")), error.to_string())?;
            Err(error.into())
        }
        TransportOutcome::Unknown(error) => {
            fs::write(work.join(format!("{name}.unknown")), error.to_string())?;
            Err(error.into())
        }
    }
}
