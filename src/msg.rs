use std::convert::{TryFrom, TryInto};
use std::iter::IntoIterator;
use metrics::Metric;
use pb_messages::{self, PbMsg};
use user_msg::UserMsg;
use errors::{ErrorKind, Error};
use pid::Pid;
use node_id::NodeId;

type Name = String;

#[derive(Debug, Clone, PartialEq)]
pub enum Msg<T: UserMsg> {
    Req(Req),
    Rpy(Rpy),
    User(T)
}

/// A request message sent to pids. Not all requests have replies.
#[derive(Debug, Clone, PartialEq)]
pub enum Req {
    GetMetrics,
    StartTimer(u64),// time in ms
    CancelTimer,
    Shutdown,
    GetProcesses(NodeId),
    GetServices(NodeId),
    GetMembership
}

/// A reply message sent to pids. 
#[derive(Debug, Clone, PartialEq)]
pub enum Rpy {
    Timeout,
    Metrics(Vec<(Name, Metric)>),
    Processes(Vec<Pid>),
    Services(Vec<Pid>),
    Members(Vec<(NodeId, bool)>),
    Error(String)
}
    
impl<T: UserMsg> TryFrom<PbMsg> for Msg<T> {
    type Error = Error;
    fn try_from(mut pb_msg: PbMsg) -> Result<Msg<T>, Error> {

        /* The user level message type */
        if pb_msg.has_user_msg() {
            let encoded = pb_msg.take_user_msg();
            return Ok(Msg::User(T::from_bytes(encoded)?));
        }

        /* Requests */

        if pb_msg.has_get_membership() {
            return Ok(Msg::Req(Req::GetMembership));
        }
        if pb_msg.has_start_timer() {
            return Ok(Msg::Req(Req::StartTimer(pb_msg.get_start_timer())));
        }
        if pb_msg.has_cancel_timer() {
            return Ok(Msg::Req(Req::CancelTimer));
        }
        if pb_msg.has_get_metrics() {
            return Ok(Msg::Req(Req::GetMetrics));
        }
        if pb_msg.has_shutdown() {
            return Ok(Msg::Req(Req::Shutdown));
        }
        if pb_msg.has_get_processes() {
            let node_id = pb_msg.take_get_processes().into();
            return Ok(Msg::Req(Req::GetProcesses(node_id)));
        }
        if pb_msg.has_get_services() {
            let node_id = pb_msg.take_get_services().into();
            return Ok(Msg::Req(Req::GetServices(node_id)));
        }

        /* Replies */
        if pb_msg.has_timeout() {
            return Ok(Msg::Rpy(Rpy::Timeout));
        }
        if pb_msg.has_error() {
            return Ok(Msg::Rpy(Rpy::Error(pb_msg.take_error())));
        }
        if pb_msg.has_metrics() {
            return Ok(Msg::Rpy(Rpy::Metrics(pb_msg.take_metrics().try_into()?)));
        }
        if pb_msg.has_processes() {
            let pids = pb_msg.take_processes()
                .take_pids()
                .into_iter()
                .map(|p| p.into()).collect();
            return Ok(Msg::Rpy(Rpy::Processes(pids)));
        }
        if pb_msg.has_services() {
            let pids = pb_msg.take_services()
                .take_pids()
                .into_iter()
                .map(|p| p.into()).collect();
            return Ok(Msg::Rpy(Rpy::Services(pids)));
        }
        if pb_msg.has_members() {
            let members = pb_msg.take_members()
                .take_members()
                .into_iter()
                .map(|mut m| (m.take_node().into(), m.get_connected())).collect();
            return Ok(Msg::Rpy(Rpy::Members(members)));
        }

        Err(ErrorKind::ProtobufDecodeError("Unknown Message received").into())
    }
}

impl<T: UserMsg> TryFrom<Msg<T>> for PbMsg {
    type Error = Error;
    fn try_from(msg: Msg<T>) -> Result<PbMsg, Error> {
        let mut pbmsg = PbMsg::new();
        match msg {
            Msg::User(user_msg) => {
                let bytes = user_msg.to_bytes()?;
                pbmsg.set_user_msg(bytes);
            },
            Msg::Req(Req::GetMembership) => {
                pbmsg.set_get_membership(true);
            },
            Msg::Req(Req::GetMetrics) => {
                pbmsg.set_get_metrics(true);
            },
            Msg::Req(Req::StartTimer(time_in_ms)) => {
                pbmsg.set_start_timer(time_in_ms);
            },
            Msg::Req(Req::CancelTimer) => {
                pbmsg.set_cancel_timer(true);
            },
            Msg::Req(Req::Shutdown) => {
                pbmsg.set_shutdown(true);
            },
            Msg::Req(Req::GetProcesses(node_id)) => {
                pbmsg.set_get_processes(node_id.into());
            },
            Msg::Req(Req::GetServices(node_id)) => {
                pbmsg.set_get_services(node_id.into());
            },
            Msg::Rpy(Rpy::Timeout) => {
                pbmsg.set_timeout(true);
            },
            Msg::Rpy(Rpy::Error(error)) => {
                pbmsg.set_error(error);
            },
            Msg::Rpy(Rpy::Metrics(metrics)) => {
                pbmsg.set_metrics(metrics.into())
            },
            Msg::Rpy(Rpy::Processes(pids)) => {
                let mut processes = pb_messages::Pids::new();
                processes.set_pids(pids.into_iter().map(|p| p.into()).collect());
                pbmsg.set_processes(processes);
            },
            Msg::Rpy(Rpy::Services(pids)) => {
                let mut services = pb_messages::Pids::new();
                services.set_pids(pids.into_iter().map(|p| p.into()).collect());
                pbmsg.set_services(services);
            },
            Msg::Rpy(Rpy::Members(members)) => {
                let mut pb_members = pb_messages::Members::new();
                pb_members.set_members(members.into_iter().map(|(node_id, connected)| {
                    let mut member = pb_messages::Member::new();
                    member.set_node(node_id.into());
                    member.set_connected(connected);
                    member
                }).collect());
                pbmsg.set_members(pb_members);
            }
        }
        Ok(pbmsg)
    }
}
