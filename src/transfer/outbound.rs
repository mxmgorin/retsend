//! An outbound send: one worker thread runs prepare → uploads → done, the UI
//! reads phase and per-file progress from shared state (the mirror image of
//! [`super::inbound`], with retsurf's downloads-worker shape).

use crate::net::client::{self, PrepareError};
use crate::net::protocol::{self, DeviceInfo, FileMeta};
use crate::net::{Wake, WakeReason};
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub use super::inbound::FileState;

/// Progress wakes are throttled to one per this interval.
const NOTIFY_EVERY: Duration = Duration::from_millis(250);

#[derive(Clone, Debug, PartialEq)]
pub enum OutboundPhase {
    /// prepare-upload sent; the peer's user is deciding.
    Waiting,
    Sending,
    Done,
    Declined,
    Cancelled,
    Failed(String),
}

pub struct OutboundFile {
    pub meta: FileMeta,
    pub path: PathBuf,
    pub state: Mutex<FileState>,
    pub sent: AtomicU64,
}

pub struct OutboundSession {
    pub peer_alias: String,
    pub files: Vec<OutboundFile>,
    pub total_bytes: u64,
    pub sent_total: AtomicU64,
    /// Set by the UI; the streaming reader errors out at the next chunk and
    /// the worker POSTs /cancel.
    pub cancel: AtomicBool,
    phase: Mutex<OutboundPhase>,
    last_wake: Mutex<Instant>,
}

impl OutboundSession {
    pub fn phase(&self) -> OutboundPhase {
        self.phase.lock().unwrap().clone()
    }

    pub fn is_finished(&self) -> bool {
        !matches!(
            self.phase(),
            OutboundPhase::Waiting | OutboundPhase::Sending
        )
    }

    pub fn done_count(&self) -> usize {
        self.files
            .iter()
            .filter(|f| *f.state.lock().unwrap() == FileState::Done)
            .count()
    }

    fn set_phase(&self, phase: OutboundPhase, wake: &dyn Wake) {
        *self.phase.lock().unwrap() = phase;
        wake.wake(WakeReason::Done);
    }

    fn maybe_wake(&self, wake: &dyn Wake) {
        let mut last = self.last_wake.lock().unwrap();
        if last.elapsed() >= NOTIFY_EVERY {
            *last = Instant::now();
            drop(last);
            wake.wake(WakeReason::Progress);
        }
    }
}

/// Build the session (stat every file) and start the worker thread.
/// `base` is `http://ip:port`; `me` is our announced identity.
pub fn spawn(
    peer_alias: String,
    base: String,
    me: DeviceInfo,
    paths: Vec<PathBuf>,
    wake: Arc<dyn Wake>,
) -> std::io::Result<Arc<OutboundSession>> {
    let mut files = Vec::with_capacity(paths.len());
    let mut total = 0u64;
    for path in paths {
        let size = std::fs::metadata(&path)?.len();
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "file".to_string());
        total += size;
        files.push(OutboundFile {
            meta: FileMeta {
                id: protocol::random_token(8),
                file_name,
                size,
                file_type: super::files::mime_for(&path).to_string(),
                sha256: None,
                preview: None,
                metadata: None,
            },
            path,
            state: Mutex::new(FileState::Pending),
            sent: AtomicU64::new(0),
        });
    }

    let session = Arc::new(OutboundSession {
        peer_alias,
        files,
        total_bytes: total,
        sent_total: AtomicU64::new(0),
        cancel: AtomicBool::new(false),
        phase: Mutex::new(OutboundPhase::Waiting),
        last_wake: Mutex::new(Instant::now()),
    });

    let worker = session.clone();
    std::thread::Builder::new()
        .name("outbound".into())
        .spawn(move || run(worker, base, me, wake))?;
    Ok(session)
}

fn run(session: Arc<OutboundSession>, base: String, me: DeviceInfo, wake: Arc<dyn Wake>) {
    let metas: Vec<FileMeta> = session.files.iter().map(|f| f.meta.clone()).collect();
    let response = match client::prepare_upload(&base, &me, &metas) {
        Ok(r) => r,
        Err(e) => {
            let phase = match e {
                PrepareError::Declined => OutboundPhase::Declined,
                PrepareError::PinRequired => {
                    OutboundPhase::Failed("peer requires a PIN (not supported yet)".into())
                }
                PrepareError::Busy => OutboundPhase::Failed("peer is busy".into()),
                // 204: the receiver already has everything.
                PrepareError::Finished => OutboundPhase::Done,
                PrepareError::Other(m) => OutboundPhase::Failed(m),
            };
            session.set_phase(phase, wake.as_ref());
            return;
        }
    };
    if session.cancel.load(Ordering::SeqCst) {
        // The user gave up while the peer was deciding.
        client::cancel(&base, &response.session_id);
        session.set_phase(OutboundPhase::Cancelled, wake.as_ref());
        return;
    }

    log::info!(
        "`{}` accepted; sending {} files",
        session.peer_alias,
        response.files.len()
    );
    session.set_phase(OutboundPhase::Sending, wake.as_ref());

    // Peer-facing agent (verification off — trust is the announced
    // fingerprint); no global timeout, uploads run as long as they need.
    let agent = client::agent(None);
    for file in &session.files {
        // The receiver may accept a subset; files without a token were
        // deselected on their end.
        let Some(token) = response.files.get(&file.meta.id) else {
            *file.state.lock().unwrap() = FileState::Failed("skipped by receiver".into());
            continue;
        };
        if session.cancel.load(Ordering::SeqCst) {
            break;
        }
        *file.state.lock().unwrap() = FileState::Receiving; // "in flight"

        let result = std::fs::File::open(&file.path)
            .map_err(|e| format!("open `{}`: {e}", file.path.display()))
            .and_then(|inner| {
                let mut reader = ProgressReader {
                    inner,
                    session: &session,
                    file,
                    wake: wake.as_ref(),
                };
                client::upload_file(
                    &agent,
                    &base,
                    &response.session_id,
                    &file.meta.id,
                    token,
                    &mut reader,
                    file.meta.size,
                )
            });
        match result {
            Ok(()) => *file.state.lock().unwrap() = FileState::Done,
            Err(message) => {
                // Roll this file's bytes back out of the overall bar.
                let sent = file.sent.swap(0, Ordering::SeqCst);
                session.sent_total.fetch_sub(sent, Ordering::SeqCst);
                *file.state.lock().unwrap() = FileState::Failed(message.clone());
                if !session.cancel.load(Ordering::SeqCst) {
                    log::warn!("upload `{}` failed: {message}", file.meta.file_name);
                }
            }
        }
        wake.wake(WakeReason::Progress);
    }

    if session.cancel.load(Ordering::SeqCst) {
        for file in &session.files {
            let mut state = file.state.lock().unwrap();
            if matches!(*state, FileState::Pending | FileState::Receiving) {
                *state = FileState::Failed("cancelled".into());
            }
        }
        client::cancel(&base, &response.session_id);
        session.set_phase(OutboundPhase::Cancelled, wake.as_ref());
    } else {
        session.set_phase(OutboundPhase::Done, wake.as_ref());
    }
}

/// Wraps the file being uploaded: counts bytes into the progress atomics,
/// throttle-wakes the UI, and aborts the request when the user cancels.
struct ProgressReader<'a> {
    inner: std::fs::File,
    session: &'a OutboundSession,
    file: &'a OutboundFile,
    wake: &'a dyn Wake,
}

impl Read for ProgressReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.session.cancel.load(Ordering::SeqCst) {
            return Err(std::io::Error::other("cancelled"));
        }
        let n = self.inner.read(buf)?;
        self.file.sent.fetch_add(n as u64, Ordering::SeqCst);
        self.session
            .sent_total
            .fetch_add(n as u64, Ordering::SeqCst);
        self.session.maybe_wake(self.wake);
        Ok(n)
    }
}
