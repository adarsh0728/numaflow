// This file is @generated by prost-build.
/// WMB is used in the KV offset timeline bucket as the value for the given processor entity key.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct Wmb {
    /// Idle is set to true if the given processor entity hasn't published anything
    /// to the offset timeline bucket in a batch processing cycle.
    /// Idle is used to signal an idle watermark.
    #[prost(bool, tag = "1")]
    pub idle: bool,
    /// Offset is the monotonically increasing index/offset of the buffer (buffer is the physical representation
    /// of the partition of the edge).
    #[prost(int64, tag = "2")]
    pub offset: i64,
    /// Watermark is tightly coupled with the offset and will be monotonically increasing for a given ProcessorEntity
    /// as the offset increases.
    /// When it is idling (Idle==true), for a given offset, the watermark can monotonically increase without offset
    /// increasing.
    #[prost(int64, tag = "3")]
    pub watermark: i64,
    /// Partition to identify the partition to which the watermark belongs.
    #[prost(int32, tag = "4")]
    pub partition: i32,
}
/// Heartbeat is used to track the active processors
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct Heartbeat {
    /// Heartbeat(current time in millis) published by the active processors.
    #[prost(int64, tag = "1")]
    pub heartbeat: i64,
}
