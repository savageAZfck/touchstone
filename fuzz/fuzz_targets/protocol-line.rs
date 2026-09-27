#![no_main]
use libfuzzer_sys::fuzz_target;
// Fuzz the NDJSON protocol line parser end-to-end: arbitrary bytes must
// never panic the harness's parsing path, whatever they contain.
fuzz_target!(|data: &[u8]| {
    let s = String::from_utf8_lossy(data);
    // Exercise the same path ExecAdapter::collect uses per line.
    let trimmed = s.trim();
    if trimmed.is_empty() || !trimmed.starts_with('{') {
        return;
    }
    let _ = serde_json::from_str::<serde_json::Value>(trimmed);
});
