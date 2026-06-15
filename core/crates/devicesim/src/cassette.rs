//! Cassettes: a recording of one MIDI session as timed, directional byte events.
//!
//! The format is newline-delimited JSON (NDJSON) so recordings stream-append and
//! diff cleanly in git. The first line is a header (`{"v":1,"meta":{…}}`); every
//! later line is one timed I/O event. Cassettes are committed under
//! `tools/cassettes/` — they are *our* derived data (not Roland documents), so they
//! are allowed in the repo.
//!
//! Their key use is a model-vs-reality oracle: replaying a recorded session's
//! host→device messages through a [`VirtualDevice`](crate::VirtualDevice) must
//! reproduce the recorded device→host bytes — which keeps the hand-built simulator
//! honest against real hardware once captures exist (Phase 3).

use serde::{Deserialize, Serialize};

/// Header metadata describing what was recorded and on what.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CassetteMeta {
    pub profile_id: String,
    /// Human firmware string (e.g. "0.2.10").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub firmware: Option<String>,
    /// Raw firmware version bytes from the Identity Reply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fw_bytes: Option<[u8; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<u8>,
    /// Where it was captured (e.g. "macos-coremidi") — delivery is platform-specific.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    /// How it was captured: "tap" (in-app, authoritative) or "log" (lossy fallback)
    /// or "synthetic" (hand-authored / generated).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recorder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Direction of a recorded message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// Host → device (a request we sent).
    Out,
    /// Device → host (a reply or an unsolicited push).
    In,
}

/// One timed, directional MIDI message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CassetteEvent {
    /// Microseconds since the start of the recording.
    pub t_us: u64,
    pub dir: Direction,
    /// Space-separated uppercase hex, matching the app's `midi.io` log format.
    pub hex: String,
    /// Chunk-boundary byte offsets, when one logical SysEx arrived as several
    /// packets (captured by the in-app tap; drives per-platform fragmentation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frag: Option<Vec<usize>>,
    /// The user action that triggered this exchange (e.g. "select_kit"), or a
    /// device-initiated marker (e.g. "hw_kit_scroll").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

impl CassetteEvent {
    /// Build an event from raw bytes (hex-encoding them).
    pub fn from_bytes(t_us: u64, dir: Direction, bytes: &[u8], action: Option<&str>) -> Self {
        Self {
            t_us,
            dir,
            hex: bytes_to_hex(bytes),
            frag: None,
            action: action.map(str::to_string),
        }
    }

    /// Decode this event's hex back to bytes.
    pub fn bytes(&self) -> Result<Vec<u8>, CassetteError> {
        parse_hex(&self.hex)
    }

    /// Whether this is a device-initiated push (annotated `hw…`), as opposed to a
    /// solicited reply.
    pub fn is_unsolicited(&self) -> bool {
        self.action.as_deref().is_some_and(|a| a.starts_with("hw"))
    }
}

/// A parsed recording.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cassette {
    pub meta: CassetteMeta,
    pub events: Vec<CassetteEvent>,
}

/// One request and the solicited replies that followed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exchange<'a> {
    pub request: &'a CassetteEvent,
    pub responses: Vec<&'a CassetteEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Header {
    #[serde(default = "default_version")]
    v: u32,
    meta: CassetteMeta,
}

fn default_version() -> u32 {
    1
}

impl Cassette {
    /// Build a cassette in memory.
    pub fn new(meta: CassetteMeta, events: Vec<CassetteEvent>) -> Self {
        Self { meta, events }
    }

    /// Parse NDJSON: a header line followed by one event per line. Blank lines are
    /// ignored.
    pub fn parse(ndjson: &str) -> Result<Self, CassetteError> {
        let mut lines = ndjson
            .lines()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty());

        let (_, header_line) = lines.next().ok_or(CassetteError::Empty)?;
        let header: Header =
            serde_json::from_str(header_line).map_err(|e| CassetteError::Json {
                line: 1,
                message: e.to_string(),
            })?;

        let mut events = Vec::new();
        for (idx, line) in lines {
            let event = serde_json::from_str(line).map_err(|e| CassetteError::Json {
                line: idx + 1,
                message: e.to_string(),
            })?;
            events.push(event);
        }
        Ok(Self {
            meta: header.meta,
            events,
        })
    }

    /// Serialize back to NDJSON (header line + one event per line).
    pub fn to_ndjson(&self) -> String {
        let header = Header {
            v: 1,
            meta: self.meta.clone(),
        };
        let mut out = serde_json::to_string(&header).expect("serialize cassette header");
        out.push('\n');
        for event in &self.events {
            out.push_str(&serde_json::to_string(event).expect("serialize cassette event"));
            out.push('\n');
        }
        out
    }

    /// Group the events into request→responses exchanges: each `Out` starts a new
    /// exchange and the following solicited `In` events are its responses.
    /// Device-initiated pushes (annotated `hw…`) are not treated as responses.
    pub fn exchanges(&self) -> Vec<Exchange<'_>> {
        let mut exchanges = Vec::new();
        let mut current: Option<Exchange> = None;
        for event in &self.events {
            match event.dir {
                Direction::Out => {
                    if let Some(prev) = current.take() {
                        exchanges.push(prev);
                    }
                    current = Some(Exchange {
                        request: event,
                        responses: Vec::new(),
                    });
                }
                Direction::In => {
                    if event.is_unsolicited() {
                        continue;
                    }
                    if let Some(ex) = current.as_mut() {
                        ex.responses.push(event);
                    }
                }
            }
        }
        if let Some(last) = current.take() {
            exchanges.push(last);
        }
        exchanges
    }
}

/// Encode bytes as space-separated uppercase hex (matching the app's log format).
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Decode space-separated hex back to bytes.
pub fn parse_hex(hex: &str) -> Result<Vec<u8>, CassetteError> {
    hex.split_whitespace()
        .map(|token| {
            u8::from_str_radix(token, 16).map_err(|_| CassetteError::Hex {
                token: token.to_string(),
            })
        })
        .collect()
}

/// Why a cassette could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CassetteError {
    /// No header line.
    Empty,
    /// A line was not valid JSON for its position.
    Json { line: usize, message: String },
    /// A hex token was not a byte.
    Hex { token: String },
}

impl std::fmt::Display for CassetteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CassetteError::Empty => write!(f, "empty cassette (no header line)"),
            CassetteError::Json { line, message } => {
                write!(f, "cassette line {line}: {message}")
            }
            CassetteError::Hex { token } => write!(f, "invalid hex byte {token:?}"),
        }
    }
}

impl std::error::Error for CassetteError {}

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY: &str = concat!(
        r#"{"v":1,"meta":{"profile_id":"roland-v31","fw_bytes":[0,2,1,0],"device_id":16,"recorder":"synthetic"}}"#,
        "\n",
        r#"{"t_us":0,"dir":"out","action":"connect","hex":"F0 7E 7F 06 01 F7"}"#,
        "\n",
        r#"{"t_us":21000,"dir":"in","hex":"F0 7E 10 06 02 41 01 06 03 00 00 02 01 00 F7"}"#,
        "\n",
    );

    #[test]
    fn hex_round_trips() {
        let bytes = [0xF0, 0x41, 0x10, 0x00, 0x7F, 0xF7];
        assert_eq!(bytes_to_hex(&bytes), "F0 41 10 00 7F F7");
        assert_eq!(parse_hex("F0 41 10 00 7F F7").unwrap(), bytes);
    }

    #[test]
    fn parses_header_and_events() {
        let cas = Cassette::parse(IDENTITY).unwrap();
        assert_eq!(cas.meta.profile_id, "roland-v31");
        assert_eq!(cas.meta.fw_bytes, Some([0, 2, 1, 0]));
        assert_eq!(cas.meta.device_id, Some(16));
        assert_eq!(cas.events.len(), 2);
        assert_eq!(cas.events[0].dir, Direction::Out);
        assert_eq!(cas.events[0].action.as_deref(), Some("connect"));
        assert_eq!(
            cas.events[1].bytes().unwrap(),
            vec![
                0xF0, 0x7E, 0x10, 0x06, 0x02, 0x41, 0x01, 0x06, 0x03, 0x00, 0x00, 0x02, 0x01, 0x00,
                0xF7
            ]
        );
    }

    #[test]
    fn groups_into_one_exchange() {
        let cas = Cassette::parse(IDENTITY).unwrap();
        let ex = cas.exchanges();
        assert_eq!(ex.len(), 1);
        assert_eq!(ex[0].request.action.as_deref(), Some("connect"));
        assert_eq!(ex[0].responses.len(), 1);
    }

    #[test]
    fn ndjson_round_trips_through_parse() {
        let cas = Cassette::parse(IDENTITY).unwrap();
        let reparsed = Cassette::parse(&cas.to_ndjson()).unwrap();
        assert_eq!(cas, reparsed);
    }

    #[test]
    fn unsolicited_pushes_are_not_responses() {
        let ndjson = concat!(
            r#"{"v":1,"meta":{"profile_id":"roland-v31"}}"#,
            "\n",
            r#"{"t_us":0,"dir":"out","hex":"F0 7E 7F 06 01 F7"}"#,
            "\n",
            r#"{"t_us":10,"dir":"in","hex":"F0 7E 10 06 02 41 01 06 03 00 00 02 01 00 F7"}"#,
            "\n",
            r#"{"t_us":99,"dir":"in","action":"hw_kit_scroll","hex":"F0 41 10 01 06 01 12 00 00 00 00 00 00 00 00 7C F7"}"#,
            "\n",
        );
        let cas = Cassette::parse(ndjson).unwrap();
        let ex = cas.exchanges();
        assert_eq!(ex.len(), 1);
        assert_eq!(
            ex[0].responses.len(),
            1,
            "the hw push is not a solicited response"
        );
    }

    #[test]
    fn empty_input_errs() {
        assert_eq!(Cassette::parse("   \n  "), Err(CassetteError::Empty));
    }
}
