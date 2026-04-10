//! Symbol broadcast cancellation protocol.
//!
//! This module provides cancellation tokens, broadcast messages, and cleanup
//! coordination for symbol stream operations. Cancellation is a protocol:
//! it propagates correctly to stop generation, abort transmissions, clean up
//! partial symbol sets, and notify peers.
//!
//! [`progress_certificate`] provides martingale-based statistical certificates
//! that cancellation drain is making progress toward quiescence.

pub mod progress_certificate;
pub mod symbol_cancel;

pub use progress_certificate::{
    CertificateVerdict, DrainPhase, EvidenceEntry, ProgressCertificate, ProgressConfig,
    ProgressObservation,
};
pub use symbol_cancel::{
    CancelBroadcastMetrics, CancelBroadcaster, CancelListener, CancelMessage, CancelSink,
    CleanupCoordinator, CleanupHandler, CleanupResult, CleanupStats, PeerId, SymbolCancelToken,
};
