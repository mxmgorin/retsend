//! File-transfer sessions. `inbound` is the receive side (M2); the outbound
//! send worker joins with the send milestone. Like `net`, nothing here
//! touches SDL — sessions report through the [`crate::net::Wake`] trait.

pub mod files;
pub mod inbound;
