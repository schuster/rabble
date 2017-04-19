use std::fmt::Debug;
use std::convert::TryFrom;
use errors::{Result, Error};
use pb_messages;


// A container type for status information for a given component
pub trait Metrics: Debug + Clone {
    fn data(&self) -> Vec<(String, Metric)>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Metric {
    Gauge(i64),
    Counter(u64)
}

/// Generate a struct: `$struct_name` from a set of metrics
///
/// Generate the impl containing the constructor, `$struct_name::new()` 
/// Generate `impl Metrics for $struct_name` constructing the Metric
/// variants returned from `$struct_name::data` based on the type of the struct fields.
macro_rules! metrics {
    ($struct_name:ident {
        $( $field:ident: $ty:ident ),+
    }) => {
        #[derive(Debug, Clone)]
        pub struct $struct_name {
            $( pub $field: $ty ),+
        }

        impl $struct_name {
            pub fn new() -> $struct_name {
                $struct_name {
                    $( $field: 0 ),+
                }
            }
        }

        impl Metrics for $struct_name {
            fn data(&self) -> Vec<(String, Metric)> {
                vec![
                    $( (stringify!($field).into(), type_to_metric!($ty)(self.$field)) ),+
                    ]
            }
        }
    }
}

macro_rules! type_to_metric {
    (i64) => { Metric::Gauge };
    (u64) => { Metric::Counter };
}

impl From<Vec<(String, Metric)>> for pb_messages::Metrics {
    fn from(metrics: Vec<(String, Metric)>) -> pb_messages::Metrics {
        let mut pb_metrics = pb_messages::Metrics::new();
        pb_metrics.set_metrics(metrics.into_iter().map(|(name, m)| {
            let mut metric = pb_messages::Metric::new();
            metric.set_name(name);
            match m {
                Metric::Gauge(val) => metric.set_gauge(val),
                Metric::Counter(val) => metric.set_counter(val)
                // TODO: Add histogram support
            }
            metric
        }).collect());
        pb_metrics
    }
}

impl TryFrom<pb_messages::Metrics> for Vec<(String, Metric)> {
    type Error = Error;

    fn try_from(mut msg: pb_messages::Metrics) -> Result<Vec<(String, Metric)>> {
        let pb_metrics = msg.take_metrics();
        let mut metrics = Vec::with_capacity(pb_metrics.len());
        for mut m in pb_metrics.into_iter() {
            if !m.has_name() {
                return Err("All metrics must have names".into());
            }
            if m.has_gauge() {
                metrics.push((m.take_name(), Metric::Gauge(m.get_gauge())))
            }
            if m.has_counter() {
                metrics.push((m.take_name(), Metric::Counter(m.get_counter())))
            }
            // TODO: Add histogram support
            return Err("No metric value set".into());
        }
        Ok(metrics)
    }
}
