//! Outbound HTTP — we are the sender: prepare-upload against the peer,
//! streamed uploads, cancel. Blocking `ureq` calls made from the outbound
//! worker thread (`crate::transfer::outbound`), never from the UI thread.

use super::protocol::{
    DeviceInfo, FileMeta, PrepareUploadRequest, PrepareUploadResponse, API_PREFIX,
};
use std::collections::BTreeMap;
use std::io::Read;
use std::time::Duration;

/// The peer's human is looking at an accept dialog — wait well past our own
/// receiver's 60 s decision deadline before giving up.
const PREPARE_TIMEOUT: Duration = Duration::from_secs(90);
/// Cancel is fire-and-forget; don't let it hang the worker.
const CANCEL_TIMEOUT: Duration = Duration::from_secs(5);

/// Why a prepare-upload didn't yield tokens. `Finished` is the spec's 204 —
/// the receiver already has everything, a success no-op.
#[derive(Debug)]
pub enum PrepareError {
    Declined,
    PinRequired,
    Busy,
    Finished,
    Other(String),
}

/// POST our file list to the peer; blocks until their user decides.
pub fn prepare_upload(
    base: &str,
    me: &DeviceInfo,
    files: &[FileMeta],
) -> Result<PrepareUploadResponse, PrepareError> {
    let request = PrepareUploadRequest {
        info: rest_info(me),
        files: files
            .iter()
            .map(|f| (f.id.clone(), f.clone()))
            .collect::<BTreeMap<_, _>>(),
    };
    let body = serde_json::to_string(&request).expect("request serializes");

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(PREPARE_TIMEOUT))
        .build()
        .into();
    let result = agent
        .post(format!("{base}{API_PREFIX}/prepare-upload"))
        .content_type("application/json")
        .send(body.as_str());

    let mut response = match result {
        Ok(r) => r,
        Err(ureq::Error::StatusCode(401)) => return Err(PrepareError::PinRequired),
        Err(ureq::Error::StatusCode(403)) => return Err(PrepareError::Declined),
        Err(ureq::Error::StatusCode(409)) => return Err(PrepareError::Busy),
        Err(ureq::Error::StatusCode(code)) => {
            return Err(PrepareError::Other(format!("peer answered {code}")))
        }
        Err(e) => return Err(PrepareError::Other(e.to_string())),
    };
    if response.status().as_u16() == 204 {
        return Err(PrepareError::Finished);
    }
    let mut text = String::new();
    response
        .body_mut()
        .as_reader()
        .take(1024 * 1024)
        .read_to_string(&mut text)
        .map_err(|e| PrepareError::Other(format!("read response: {e}")))?;
    serde_json::from_str(&text).map_err(|e| PrepareError::Other(format!("parse response: {e}")))
}

/// Stream one file's bytes. The explicit Content-Length switches ureq to a
/// sized (non-chunked) body, which every LocalSend receiver expects. `body`
/// is typically a progress-counting reader that errors out on cancel.
pub fn upload_file(
    agent: &ureq::Agent,
    base: &str,
    session_id: &str,
    file_id: &str,
    token: &str,
    body: &mut dyn Read,
    size: u64,
) -> Result<(), String> {
    let url =
        format!("{base}{API_PREFIX}/upload?sessionId={session_id}&fileId={file_id}&token={token}");
    agent
        .post(url)
        .header("Content-Length", size.to_string())
        .content_type("application/octet-stream")
        .send(ureq::SendBody::from_reader(body))
        .map(|_| ())
        .map_err(|e| match e {
            ureq::Error::StatusCode(code) => format!("peer answered {code}"),
            e => e.to_string(),
        })
}

/// Best-effort session cancel; failures only get logged — the peer's idle
/// timeout cleans up regardless.
pub fn cancel(base: &str, session_id: &str) {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(CANCEL_TIMEOUT))
        .build()
        .into();
    let url = format!("{base}{API_PREFIX}/cancel?sessionId={session_id}");
    if let Err(e) = agent.post(url).send(()) {
        log::debug!("cancel {session_id}: {e}");
    }
}

/// Our identity for REST bodies: `announce` is multicast-only.
fn rest_info(me: &DeviceInfo) -> DeviceInfo {
    let mut info = me.clone();
    info.announce = None;
    info
}
