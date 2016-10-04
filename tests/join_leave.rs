//! Test cluster joining and leaving

extern crate amy;
extern crate rabble;

#[macro_use]
extern crate assert_matches;
extern crate rustc_serialize;

#[macro_use]
extern crate slog;
extern crate slog_stdlog;
extern crate slog_envlogger;
extern crate slog_term;
extern crate log;
extern crate time;

mod utils;

use std::{str, thread};
use std::thread::JoinHandle;
use amy::{Poller, Receiver, Sender};
use slog::DrainExt;
use time::{SteadyTime, Duration};

use utils::messages::*;
use utils::{
    wait_for,
    create_node_ids,
    start_nodes
};

use rabble::{
    Pid,
    NodeId,
    Envelope,
    Msg,
    ClusterStatus,
    MsgpackSerializer,
    Serialize,
    Node,
    CorrelationId
};

const NUM_NODES: usize = 3;

#[test]
fn join_leave() {
    let node_ids = create_node_ids(NUM_NODES);
    let test_pid = Pid {
        name: "test-runner".to_string(),
        group: None,
        node: node_ids[0].clone()
    };

    let (nodes, mut handles) = start_nodes(NUM_NODES);

    // We create an amy channel so that we can pretend this test is a system service.
    // We register the sender with all nodes so that we can check the responses to admin calls
    // like node.get_cluster_status().
    let mut poller = Poller::new().unwrap();
    let (test_tx, test_rx) = poller.get_registrar().channel().unwrap();
    for node in &nodes {
        node.register_system_thread(&test_pid, &test_tx).unwrap();
    }

    // join node1 to node2
    // Wait for the cluster status of both nodes to show they are connected
    nodes[0].join(&nodes[1].id).unwrap();
    assert!(wait_for_cluster_status(&nodes[0], &test_pid, &test_rx, 1));
    assert!(wait_for_cluster_status(&nodes[1], &test_pid, &test_rx, 1));

    // Join node1 to node3. This will cause a delta to be sent from node1 to node2. Node3 will also
    // connect to node2 and send it's members, since it will learn of node2 from node1. Either way
    // all nodes should stabilize as knowing about each other.
    nodes[0].join(&nodes[2].id).unwrap();
    for node in &nodes {
        assert!(wait_for_cluster_status(&node, &test_pid, &test_rx, 2));
    }

    // Remove node2 from the cluster. This will cause a delta of the remove to be broadcast to node1
    // 1 and node3. Note that the request is sent to node1, not the node that is leaving.
    nodes[0].leave(&nodes[1].id).unwrap();
    assert!(wait_for_cluster_status(&nodes[0], &test_pid, &test_rx, 1));
    assert!(wait_for_cluster_status(&nodes[2], &test_pid, &test_rx, 1));


    // Remove node1 from the cluster. This request goest to node1. It's possible in production that
    // the broadcast doesn't make it to node3 before node1 disconnects from node3 due to the
    // membership check on the next tick that removes connections.
    // TODO: make that work
    nodes[0].leave(&nodes[0].id).unwrap();
    assert!(wait_for_cluster_status(&nodes[0], &test_pid, &test_rx, 0));
    assert!(wait_for_cluster_status(&nodes[2], &test_pid, &test_rx, 0));
}

fn wait_for_cluster_status(node: &Node<RabbleUserMsg>,
                           test_pid: &Pid,
                           test_rx: &Receiver<Envelope<RabbleUserMsg>>,
                           num_connected: usize) -> bool
{
    let sleep_time = Duration::milliseconds(10);
    let timeout = Duration::seconds(5);
    wait_for(sleep_time, timeout, || {
        let correlation_id = CorrelationId::pid(test_pid.clone());
        node.cluster_status(correlation_id.clone()).unwrap();
        if let Ok(envelope) = test_rx.try_recv() {
            if let Msg::ClusterStatus(ClusterStatus{connected, members, ..}) = envelope.msg {
                if connected.len() == num_connected {
                    return true;
                }
            }
        }
        false
    })
}
