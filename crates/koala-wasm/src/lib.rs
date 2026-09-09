//! Thin WASI adapter over the Koala Rust planner core.
//!
//! The adapter owns transport only. Search remains in `planner`; no planning
//! algorithm is reimplemented here and no operation has actuation authority.

use planner::{solve_grounded_json, SolveOptions};
use serde::Deserialize;
use serde_json::{json, Value};
use std::mem;
use std::slice;

#[derive(Debug, Deserialize)]
struct Request {
    op: String,
    #[serde(default)]
    grounded_problem: Option<Value>,
    #[serde(default)]
    options: Option<SolveOptions>,
}

fn dispatch(input: &[u8]) -> Value {
    let request: Request = match serde_json::from_slice(input) {
        Ok(request) => request,
        Err(error) => {
            return json!({
                "ok": false,
                "error": {
                    "kind": "invalid_request",
                    "message": error.to_string()
                }
            })
        }
    };

    match request.op.as_str() {
        "readiness" => json!({
            "ok": true,
            "engine": "koala",
            "transport": "wasm32-wasip1",
            "input_contract": "grounded-json",
            "select_only": true,
            "operations": ["readiness", "solve"]
        }),
        "solve" => {
            let grounded_problem = match request.grounded_problem {
                Some(problem) => problem,
                None => {
                    return json!({
                        "ok": false,
                        "error": {
                            "kind": "missing_grounded_problem",
                            "message": "solve requires grounded_problem"
                        }
                    })
                }
            };

            let grounded_json = match serde_json::to_string(&grounded_problem) {
                Ok(json) => json,
                Err(error) => {
                    return json!({
                        "ok": false,
                        "error": {
                            "kind": "invalid_grounded_problem",
                            "message": error.to_string()
                        }
                    })
                }
            };

            match solve_grounded_json(&grounded_json, request.options.unwrap_or_default()) {
                Ok(report) => json!({"ok": true, "report": report}),
                Err(message) => json!({
                    "ok": false,
                    "error": {
                        "kind": "invalid_grounded_problem",
                        "message": message
                    }
                }),
            }
        }
        other => json!({
            "ok": false,
            "error": {
                "kind": "unknown_operation",
                "message": format!("unknown operation '{other}'")
            }
        }),
    }
}

/// Allocate exactly `len` guest bytes for a host request.
///
/// The allocation is a boxed slice so its deallocation contract is completely
/// determined by the returned pointer and requested length.
#[no_mangle]
pub extern "C" fn koala_alloc(len: u32) -> u32 {
    if len == 0 {
        return 0;
    }

    let mut buffer = vec![0_u8; len as usize].into_boxed_slice();
    let ptr = buffer.as_mut_ptr() as usize as u32;
    mem::forget(buffer);
    ptr
}

/// Consume one JSON request and return `(out_ptr << 32) | out_len`.
///
/// The request allocation is consumed by this function. The host owns only the
/// returned response allocation and must release it with `koala_dealloc`.
#[no_mangle]
pub extern "C" fn koala_call(ptr: u32, len: u32) -> u64 {
    if ptr == 0 || len == 0 {
        return encode_response(json!({
            "ok": false,
            "error": {
                "kind": "invalid_buffer",
                "message": "request pointer and length must both be non-zero"
            }
        }));
    }

    let input = unsafe {
        let raw = slice::from_raw_parts_mut(ptr as usize as *mut u8, len as usize);
        Box::from_raw(raw)
    };

    encode_response(dispatch(&input))
}

/// Free a response allocation returned by `koala_call`.
#[no_mangle]
pub extern "C" fn koala_dealloc(ptr: u32, len: u32) {
    if ptr == 0 || len == 0 {
        return;
    }

    unsafe {
        let raw = slice::from_raw_parts_mut(ptr as usize as *mut u8, len as usize);
        drop(Box::from_raw(raw));
    }
}

fn encode_response(value: Value) -> u64 {
    let bytes = serde_json::to_vec(&value).unwrap_or_else(|error| {
        format!(
            "{{\"ok\":false,\"error\":{{\"kind\":\"serialization\",\"message\":{:?}}}}}",
            error.to_string()
        )
        .into_bytes()
    });

    if bytes.is_empty() {
        return 0;
    }

    let mut buffer = bytes.into_boxed_slice();
    let len = buffer.len() as u32;
    let ptr = buffer.as_mut_ptr() as usize as u32;
    mem::forget(buffer);

    ((ptr as u64) << 32) | len as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_is_explicitly_select_only() {
        let response = dispatch(br#"{"op":"readiness"}"#);
        assert_eq!(response["ok"], true);
        assert_eq!(response["select_only"], true);
        assert_eq!(response["input_contract"], "grounded-json");
    }

    #[test]
    fn malformed_requests_are_typed_refusals() {
        let response = dispatch(b"not-json");
        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["kind"], "invalid_request");
    }

    #[test]
    fn raw_hddl_is_not_silently_claimed_as_supported() {
        let response = dispatch(br#"{"op":"solve"}"#);
        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["kind"], "missing_grounded_problem");
    }
}
