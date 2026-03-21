use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};

static STAGE_TRACE_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
static STAGE_TRACE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn stage_trace_path() -> Option<&'static PathBuf> {
    STAGE_TRACE_PATH
        .get_or_init(|| std::env::var("VPNNODE_STAGE_TRACE_PATH").ok().map(PathBuf::from))
        .as_ref()
}

fn stage_trace_lock() -> &'static Mutex<()> {
    STAGE_TRACE_LOCK.get_or_init(|| Mutex::new(()))
}

pub fn enabled() -> bool {
    stage_trace_path().is_some()
}

pub fn emit(mut value: Value) {
    let Some(path) = stage_trace_path() else {
        return;
    };
    let Value::Object(ref mut object) = value else {
        return;
    };
    object
        .entry("timestamp_unix_ms".to_string())
        .or_insert_with(|| Value::from(now_unix_ms() as u64));
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(line) = serde_json::to_string(&value) else {
        return;
    };
    let _guard = stage_trace_lock().lock().unwrap();
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let _ = writeln!(file, "{line}");
}

pub fn event(component: &str, stage: &str) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert("component".to_string(), Value::from(component));
    object.insert("stage".to_string(), Value::from(stage));
    object
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}
