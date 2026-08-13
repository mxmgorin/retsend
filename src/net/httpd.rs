//! Minimal HTTP/1.1 server primitives — just enough for the LocalSend surface
//! (4 POSTs + 1 GET). Hand-rolled instead of a
//! dependency: the parser is ~150 lines, testable over `io::Cursor`, and stays
//! agnostic of the transport (`BufRead`), which is the seam the HTTPS
//! milestone slots a rustls stream into.
//!
//! Simplifications, deliberate on a LAN protocol:
//! - every response is `Connection: close`; clients reconnect per request
//! - of the transfer codings only `chunked` is decoded; the rest get 501

use std::io::{BufRead, Read, Write};

const MAX_REQUEST_LINE: usize = 8 * 1024;
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_HEADERS: usize = 64;
/// A chunk's size line, and the trailers that share its framing.
const MAX_CHUNK_LINE: usize = 1024;

#[derive(Debug)]
pub struct Request {
    pub method: String,
    /// Percent-decoded path, no query string.
    pub path: String,
    /// Percent-decoded query pairs in order of appearance.
    pub query: Vec<(String, String)>,
    /// Lowercased names.
    pub headers: Vec<(String, String)>,
    pub content_length: u64,
    /// Chunked body: the length is in the framing, `content_length` is unset.
    pub chunked: bool,
    /// The client sent `Expect: 100-continue` and is waiting for our go-ahead
    /// before transmitting the body (curl does this for large POSTs).
    pub expects_continue: bool,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == &name.to_ascii_lowercase())
            .map(|(_, v)| v.as_str())
    }

    pub fn query_param(&self, name: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Parse errors carry the status code the connection should die with.
#[derive(Debug)]
pub struct ParseError {
    pub status: u16,
    pub message: String,
}

impl ParseError {
    fn new(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

/// Read the request head (request line + headers) from `reader`. The body, if
/// any, remains unread — stream it afterwards via [`body_reader`].
pub fn parse_request(reader: &mut impl BufRead) -> Result<Request, ParseError> {
    let line = read_line(reader, MAX_REQUEST_LINE)
        .map_err(|e| ParseError::new(400, format!("request line: {e}")))?;
    let mut parts = line.split_whitespace();
    let (method, target, version) = match (parts.next(), parts.next(), parts.next()) {
        (Some(m), Some(t), Some(v)) => (m, t, v),
        _ => {
            return Err(ParseError::new(
                400,
                format!("malformed request line `{line}`"),
            ))
        }
    };
    if !version.starts_with("HTTP/1.") {
        return Err(ParseError::new(
            505,
            format!("unsupported version `{version}`"),
        ));
    }

    let (raw_path, raw_query) = match target.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (target, None),
    };
    let path = percent_decode(raw_path)
        .ok_or_else(|| ParseError::new(400, "bad percent-encoding in path"))?;
    let mut query = Vec::new();
    if let Some(raw) = raw_query {
        for pair in raw.split('&').filter(|p| !p.is_empty()) {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            let k = percent_decode(k)
                .ok_or_else(|| ParseError::new(400, "bad percent-encoding in query"))?;
            let v = percent_decode(v)
                .ok_or_else(|| ParseError::new(400, "bad percent-encoding in query"))?;
            query.push((k, v));
        }
    }

    let mut headers = Vec::new();
    let mut header_bytes = 0usize;
    loop {
        let line = read_line(reader, MAX_HEADER_BYTES)
            .map_err(|e| ParseError::new(400, format!("headers: {e}")))?;
        if line.is_empty() {
            break;
        }
        header_bytes += line.len();
        if header_bytes > MAX_HEADER_BYTES || headers.len() >= MAX_HEADERS {
            return Err(ParseError::new(431, "headers too large"));
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| ParseError::new(400, format!("malformed header `{line}`")))?;
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
    }

    let mut request = Request {
        method: method.to_string(),
        path,
        query,
        headers,
        content_length: 0,
        chunked: false,
        expects_continue: false,
    };

    // Only the last coding frames the body. LocalSend 1.18+ streams uploads
    // through reqwest, which sends them chunked.
    if let Some(value) = request.header("transfer-encoding") {
        let last = match value.rsplit_once(',') {
            Some((_, last)) => last,
            None => value,
        };
        let last = last.trim();
        if last.eq_ignore_ascii_case("chunked") {
            request.chunked = true;
        } else if !last.eq_ignore_ascii_case("identity") {
            return Err(ParseError::new(
                501,
                format!("transfer-encoding `{last}` unsupported"),
            ));
        }
    }
    // RFC 9112: a Content-Length sent beside chunked framing is ignored.
    if let Some(v) = request
        .header("content-length")
        .filter(|_| !request.chunked)
    {
        request.content_length = v
            .parse::<u64>()
            .map_err(|_| ParseError::new(400, format!("bad content-length `{v}`")))?;
    }
    request.expects_continue = request
        .header("expect")
        .is_some_and(|v| v.eq_ignore_ascii_case("100-continue"));

    Ok(request)
}

/// The request body: `Content-Length` bytes, or the dechunked stream. Send
/// [`write_continue`] first when `expects_continue` is set, or the client will
/// sit out its timeout. A chunked body is bounded only by its own framing, so
/// callers buffering it in memory must cap it themselves.
pub fn body_reader<'a, R: BufRead>(reader: &'a mut R, request: &Request) -> impl Read + 'a {
    if request.chunked {
        Body::Chunked(ChunkedReader {
            inner: reader,
            remaining: 0,
            done: false,
        })
    } else {
        Body::Sized(reader.take(request.content_length))
    }
}

enum Body<'a, R: BufRead> {
    Sized(std::io::Take<&'a mut R>),
    Chunked(ChunkedReader<'a, R>),
}

impl<R: BufRead> Read for Body<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Sized(r) => r.read(buf),
            Self::Chunked(r) => r.read(buf),
        }
    }
}

/// Decodes `Transfer-Encoding: chunked`. Extensions and trailers are dropped —
/// nothing in the LocalSend surface reads them.
struct ChunkedReader<'a, R: BufRead> {
    inner: &'a mut R,
    /// Bytes left in the chunk being streamed.
    remaining: u64,
    /// The terminating zero-length chunk arrived; reads are EOF from here.
    done: bool,
}

impl<R: BufRead> ChunkedReader<'_, R> {
    /// Consume a size line, leaving `remaining` set.
    fn start_chunk(&mut self) -> std::io::Result<()> {
        let line = read_line(self.inner, MAX_CHUNK_LINE)?;
        let size = match line.split_once(';') {
            Some((size, _extensions)) => size,
            None => &line,
        };
        let size = u64::from_str_radix(size.trim(), 16)
            .map_err(|_| malformed(format!("bad chunk size `{size}`")))?;
        if size == 0 {
            self.consume_trailers()?;
            self.done = true;
        }
        self.remaining = size;
        Ok(())
    }

    /// The CRLF that closes a chunk's data.
    fn end_chunk(&mut self) -> std::io::Result<()> {
        match read_line(self.inner, MAX_CHUNK_LINE)?.is_empty() {
            true => Ok(()),
            false => Err(malformed("chunk data not CRLF-terminated")),
        }
    }

    /// Trailers run until a blank line, capped like headers so a peer can't
    /// stream them forever.
    fn consume_trailers(&mut self) -> std::io::Result<()> {
        for _ in 0..MAX_HEADERS {
            if read_line(self.inner, MAX_CHUNK_LINE)?.is_empty() {
                return Ok(());
            }
        }
        Err(malformed("too many trailers"))
    }
}

impl<R: BufRead> Read for ChunkedReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        while self.remaining == 0 {
            if self.done {
                return Ok(0);
            }
            self.start_chunk()?;
        }
        // Compared as u64: a 32-bit usize would truncate the chunk size.
        let want = self.remaining.min(buf.len() as u64) as usize;
        let read = self.inner.read(&mut buf[..want])?;
        if read == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "connection closed mid-chunk",
            ));
        }
        self.remaining -= read as u64;
        if self.remaining == 0 {
            self.end_chunk()?;
        }
        Ok(read)
    }
}

fn malformed(message: impl Into<String>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message.into())
}

pub fn write_continue(stream: &mut impl Write) -> std::io::Result<()> {
    stream.write_all(b"HTTP/1.1 100 Continue\r\n\r\n")
}

pub fn respond_empty(stream: &mut impl Write, status: u16) -> std::io::Result<()> {
    respond(stream, status, None)
}

pub fn respond_json(stream: &mut impl Write, status: u16, json: &str) -> std::io::Result<()> {
    respond(stream, status, Some(json))
}

fn respond(stream: &mut impl Write, status: u16, json: Option<&str>) -> std::io::Result<()> {
    let reason = reason_phrase(status);
    let body = json.unwrap_or("");
    let content_type = if json.is_some() {
        "Content-Type: application/json\r\n"
    } else {
        ""
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\n{content_type}Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    stream.flush()
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        505 => "HTTP Version Not Supported",
        _ => "",
    }
}

/// One CRLF- (or bare-LF-) terminated line, without the terminator, capped at
/// `max` bytes. EOF before any byte is an error (the connection died).
fn read_line(reader: &mut impl BufRead, max: usize) -> std::io::Result<String> {
    let mut buf = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        match reader.read(&mut byte) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "connection closed mid-line",
                ))
            }
            Ok(_) => {
                if byte[0] == b'\n' {
                    break;
                }
                buf.push(byte[0]);
                if buf.len() > max {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "line too long",
                    ));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    if buf.last() == Some(&b'\r') {
        buf.pop();
    }
    String::from_utf8(buf)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "non-UTF8 line"))
}

/// `%XX` decoding (plus `+` as space, the form-encoding convention query
/// strings use). `None` on truncated/invalid escapes.
fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let hi = char::from(*bytes.get(i + 1)?).to_digit(16)?;
                let lo = char::from(*bytes.get(i + 2)?).to_digit(16)?;
                out.push((hi * 16 + lo) as u8);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn parse(raw: &str) -> Result<Request, ParseError> {
        parse_request(&mut Cursor::new(raw.as_bytes()))
    }

    #[test]
    fn parses_post_with_body() {
        let raw = "POST /api/localsend/v2/register HTTP/1.1\r\n\
                   Host: x\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}";
        let mut cursor = Cursor::new(raw.as_bytes());
        let req = parse_request(&mut cursor).unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/api/localsend/v2/register");
        assert_eq!(req.content_length, 2);
        let mut body = String::new();
        body_reader(&mut cursor, &req)
            .read_to_string(&mut body)
            .unwrap();
        assert_eq!(body, "{}");
    }

    #[test]
    fn parses_query_params() {
        let req =
            parse("POST /upload?sessionId=abc&fileId=f%201&token=t+x HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(req.query_param("sessionId"), Some("abc"));
        assert_eq!(req.query_param("fileId"), Some("f 1"));
        assert_eq!(req.query_param("token"), Some("t x"));
    }

    /// LocalSend 1.18+ streams uploads through reqwest, which frames them
    /// chunked rather than with a Content-Length.
    #[test]
    fn decodes_a_chunked_body() {
        let raw = "POST /x HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n\
                   4\r\nWiki\r\n5;ext=1\r\npedia\r\n0\r\n\r\n";
        let mut cursor = Cursor::new(raw.as_bytes());
        let req = parse_request(&mut cursor).unwrap();
        assert!(req.chunked);
        let mut body = String::new();
        body_reader(&mut cursor, &req)
            .read_to_string(&mut body)
            .unwrap();
        assert_eq!(body, "Wikipedia");
    }

    #[test]
    fn chunked_trailers_are_consumed() {
        let raw = "POST /x HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n\
                   2\r\nhi\r\n0\r\nX-Checksum: abc\r\n\r\n";
        let mut cursor = Cursor::new(raw.as_bytes());
        let req = parse_request(&mut cursor).unwrap();
        let mut body = String::new();
        body_reader(&mut cursor, &req)
            .read_to_string(&mut body)
            .unwrap();
        assert_eq!(body, "hi");
    }

    /// Chunked framing wins, so a Content-Length beside it must not bound the
    /// body — trusting it would truncate the upload.
    #[test]
    fn chunked_ignores_content_length() {
        let raw = "POST /x HTTP/1.1\r\nTransfer-Encoding: chunked\r\nContent-Length: 2\r\n\r\n\
                   9\r\nWikipedia\r\n0\r\n\r\n";
        let mut cursor = Cursor::new(raw.as_bytes());
        let req = parse_request(&mut cursor).unwrap();
        assert_eq!(req.content_length, 0);
        let mut body = String::new();
        body_reader(&mut cursor, &req)
            .read_to_string(&mut body)
            .unwrap();
        assert_eq!(body, "Wikipedia");
    }

    #[test]
    fn a_truncated_chunked_body_errors() {
        let raw = "POST /x HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n9\r\nWiki";
        let mut cursor = Cursor::new(raw.as_bytes());
        let req = parse_request(&mut cursor).unwrap();
        let mut body = String::new();
        assert!(body_reader(&mut cursor, &req)
            .read_to_string(&mut body)
            .is_err());
    }

    #[test]
    fn unknown_transfer_coding_gets_501() {
        let err = parse("POST /x HTTP/1.1\r\nTransfer-Encoding: gzip\r\n\r\n").unwrap_err();
        assert_eq!(err.status, 501);
    }

    #[test]
    fn missing_content_length_means_empty_body() {
        let req = parse("GET /api/localsend/v2/info HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(req.content_length, 0);
    }

    #[test]
    fn detects_expect_continue() {
        let req =
            parse("POST /x HTTP/1.1\r\nExpect: 100-continue\r\nContent-Length: 5\r\n\r\n").unwrap();
        assert!(req.expects_continue);
    }

    #[test]
    fn oversized_headers_get_431() {
        let mut raw = String::from("GET / HTTP/1.1\r\n");
        for i in 0..100 {
            raw.push_str(&format!("X-Filler-{i}: {}\r\n", "y".repeat(300)));
        }
        raw.push_str("\r\n");
        let err = parse(&raw).unwrap_err();
        assert_eq!(err.status, 431);
    }

    #[test]
    fn bad_request_line_gets_400() {
        assert_eq!(parse("NONSENSE\r\n\r\n").unwrap_err().status, 400);
        assert_eq!(parse("GET / SPDY/3\r\n\r\n").unwrap_err().status, 505);
    }

    #[test]
    fn responses_are_well_formed() {
        let mut out = Vec::new();
        respond_json(&mut out, 200, "{\"ok\":true}").unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(text.contains("Content-Length: 11\r\n"));
        assert!(text.contains("Connection: close\r\n"));
        assert!(text.ends_with("{\"ok\":true}"));

        let mut out = Vec::new();
        respond_empty(&mut out, 403).unwrap();
        assert!(String::from_utf8(out)
            .unwrap()
            .starts_with("HTTP/1.1 403 Forbidden\r\n"));
    }
}
