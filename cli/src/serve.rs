//! `dyn serve`: a model as a web service. A JSON API read from the manifest, over plain HTTP/1.1 on `std::net` (no new
//! dependencies). Built from the specification of the task, not from a reference script.
//!
//! * `GET /manifest`: the manifest of the model, with its version.
//! * `POST /run`: `{"scenario": "Name"}` and/or `{"values": {"Lever": 3}}`; `{"series": true}` adds the output series.
//! * `POST /batch`: `{"runs": [ {...}, {...} ]}` (up to 100 runs, each as for `/run`).
//! * `GET /openapi.json`: the OpenAPI 3.0 description (the only call without a token).
//!
//! A name that is not a lever of the manifest is refused (400); a value outside the tested range of its lever is refused
//! (422). Outputs are read at the validity horizon; the value at the end of the run, when that is later, comes under
//! `beyond_horizon` with a label. Every call is logged (one JSON line) with the version of the model. Every call but the
//! description needs `Authorization: Bearer TOKEN`. The default address is 127.0.0.1.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use dynamist_core::expr;
use dynamist_core::manifest::{self, Goal};
use dynamist_core::model::Model;
use dynamist_core::sanity;
use serde_json::{Value, json};

pub struct Config {
    pub bind: String,
    pub port: u16,
    pub token: String,
    /// a file that every call is appended to (also written to standard error when None)
    pub log: Option<String>,
}

/// A running server: where it listens, and a way to stop it.
pub struct Server {
    pub addr: SocketAddr,
    stop: Arc<std::sync::atomic::AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    pub fn stop(mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = TcpStream::connect(self.addr);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
    /// Serve until the process ends.
    pub fn wait(mut self) {
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// The version of a model: a hash of its content, so that a log line names exactly what answered.
pub fn version_of(m: &Model) -> String {
    let text = serde_json::to_string(m).unwrap_or_default();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{}@{:016x}", if m.title.trim().is_empty() { "model" } else { m.title.trim() }, h)
}

struct Ctx {
    model: Model,
    version: String,
    token: String,
    log: Mutex<Option<std::fs::File>>,
    to_stderr: bool,
}

fn same(a: &str, b: &str) -> bool {
    // constant time over the length of the longer one
    let (x, y) = (a.as_bytes(), b.as_bytes());
    let mut d = (x.len() ^ y.len()) as u8 as usize;
    for i in 0..x.len().max(y.len()) {
        d |= (x.get(i).copied().unwrap_or(0) ^ y.get(i).copied().unwrap_or(1)) as usize;
    }
    d == 0
}

fn read_time(m: &Model) -> f64 {
    let end = m.settings.final_time;
    sanity::horizon(m).map_or(end, |h| h.min(end))
}

fn output_names(m: &Model) -> Vec<String> {
    dynamist_core::workbook::outputs_of(m)
}

fn value_at(run: &dynamist_core::sim::Run, name: &str, t: f64) -> Option<f64> {
    manifest::goal_value_raw(run, &Goal { name: name.into(), output: name.into(), measure: format!("at:{t}"), ..Default::default() })
}

/// One run: a scenario and/or lever values, refused if a name is not a lever or a value is outside its tested range.
fn run_one(c: &Ctx, body: &Value) -> Result<Value, (u16, String)> {
    let m = &c.model;
    let obj = body.as_object().ok_or((400, "the body must be a JSON object".to_string()))?;
    for k in obj.keys() {
        if !["scenario", "values", "series"].contains(&k.as_str()) {
            return Err((400, format!("“{k}” is not a field of a run (scenario, values, series)")));
        }
    }
    let mut values: BTreeMap<String, f64> = BTreeMap::new();
    let mut scenario_name: Option<String> = None;
    if let Some(s) = obj.get("scenario") {
        let name = s.as_str().ok_or((400, "scenario must be a name".to_string()))?;
        let sc = m.manifest.scenarios.iter().find(|x| expr::normalize(&x.name) == expr::normalize(name)).ok_or((404, format!("there is no scenario called “{name}”")))?;
        scenario_name = Some(sc.name.clone());
        values.extend(sc.values.clone());
    }
    if let Some(v) = obj.get("values") {
        let o = v.as_object().ok_or((400, "values must be an object of lever names and numbers".to_string()))?;
        for (k, x) in o {
            let lever = m.manifest.levers.iter().find(|l| expr::normalize(&l.name) == expr::normalize(k)).ok_or((400, format!("“{k}” is not a lever of this model; the levers are {}", m.manifest.levers.iter().map(|l| l.name.as_str()).collect::<Vec<_>>().join(", "))))?;
            let x = x.as_f64().filter(|x| x.is_finite()).ok_or((400, format!("the value of “{k}” must be a number")))?;
            if x < lever.min || x > lever.max {
                return Err((422, format!("“{}” = {x} is outside the tested range {} to {}", lever.name, lever.min, lever.max)));
            }
            values.insert(lever.name.clone(), x);
        }
    }
    let model = sanity::with_values(m, &values).map_err(|e| (400, e))?;
    let run = dynamist_core::runner::run_with(&model, &[]).map_err(|e| (500, format!("the model does not run: {e}")))?;
    let end = m.settings.final_time;
    let at = read_time(m);
    let outs = output_names(m);
    let units = |n: &str| m.manifest.outputs.iter().find(|o| expr::normalize(&o.variable) == expr::normalize(n)).map(|o| o.units.clone()).filter(|u| !u.is_empty()).or_else(|| m.by_name(&expr::normalize(n)).map(|v| v.units.clone())).unwrap_or_default();
    let table = |t: f64| outs.iter().map(|o| json!({"variable": o, "units": units(o), "value": value_at(&run, o, t)})).collect::<Vec<_>>();
    let horizon = sanity::horizon(m);
    let mut out = json!({
        "model_version": c.version,
        "scenario": scenario_name,
        "values": values,
        "time_units": m.settings.time_units,
        "read_at": at,
        "validity_horizon": horizon,
        "outputs": table(at),
    });
    if end > at + 1e-9 {
        out["beyond_horizon"] = json!({"label": "beyond the validity horizon: not vouched for", "time": end, "outputs": table(end)});
    }
    if obj.get("series").and_then(|s| s.as_bool()).unwrap_or(false) {
        let series: BTreeMap<&String, &Vec<f64>> = outs.iter().filter_map(|o| run.get(o).map(|s| (o, s))).collect();
        out["series"] = json!({"times": run.times, "values": series, "beyond_horizon_from": horizon.filter(|h| *h < end)});
    }
    Ok(out)
}

fn batch(c: &Ctx, body: &Value) -> Result<Value, (u16, String)> {
    let runs = body.get("runs").and_then(|r| r.as_array()).ok_or((400, "write {\"runs\": [ ... ]}".to_string()))?;
    if runs.len() > 100 {
        return Err((400, "at most 100 runs in a batch".into()));
    }
    let results: Vec<Value> = runs
        .iter()
        .enumerate()
        .map(|(i, r)| match run_one(c, r) {
            Ok(v) => json!({"index": i, "status": 200, "result": v}),
            Err((s, e)) => json!({"index": i, "status": s, "error": e}),
        })
        .collect();
    Ok(json!({"model_version": c.version, "results": results}))
}

/// The OpenAPI 3.0 description of this model's service.
pub fn openapi(m: &Model, version: &str) -> Value {
    let levers: Vec<Value> = m.manifest.levers.iter().map(|l| json!({"name": l.name, "units": l.units, "minimum": l.min, "maximum": l.max})).collect();
    let run_body = json!({
        "type": "object", "additionalProperties": false,
        "properties": {
            "scenario": {"type": "string", "description": "A named scenario of the manifest", "enum": m.manifest.scenarios.iter().map(|s| s.name.clone()).collect::<Vec<_>>()},
            "values": {"type": "object", "description": "Lever values, each inside its tested range", "properties": m.manifest.levers.iter().map(|l| (l.name.clone(), json!({"type": "number", "minimum": l.min, "maximum": l.max, "description": l.units}))).collect::<serde_json::Map<String, Value>>(), "additionalProperties": false},
            "series": {"type": "boolean", "description": "Also return the output series"}
        }
    });
    let err = json!({"description": "Refused", "content": {"application/json": {"schema": {"type": "object", "properties": {"error": {"type": "string"}}}}}});
    json!({
        "openapi": "3.0.3",
        "info": {"title": if m.manifest.title.is_empty() { m.title.clone() } else { m.manifest.title.clone() }, "description": m.manifest.description, "version": version},
        "servers": [{"url": "http://127.0.0.1:8080"}],
        "security": [{"bearer": []}],
        "x-levers": levers,
        "paths": {
            "/manifest": {"get": {"summary": "The manifest of the model", "responses": {"200": {"description": "The manifest and the model version"}, "401": err}}},
            "/run": {"post": {"summary": "Run a scenario or lever values; outputs are read at the validity horizon, later values come under beyond_horizon", "requestBody": {"required": true, "content": {"application/json": {"schema": run_body}}}, "responses": {"200": {"description": "The outputs"}, "400": err, "401": err, "404": err, "422": err}}},
            "/batch": {"post": {"summary": "Up to 100 runs", "requestBody": {"required": true, "content": {"application/json": {"schema": {"type": "object", "properties": {"runs": {"type": "array", "maxItems": 100, "items": run_body}}, "required": ["runs"]}}}}, "responses": {"200": {"description": "One result or error per run"}, "400": err, "401": err}}},
            "/openapi.json": {"get": {"summary": "This description", "security": [], "responses": {"200": {"description": "OpenAPI 3.0 document"}}}}
        },
        "components": {"securitySchemes": {"bearer": {"type": "http", "scheme": "bearer"}}}
    })
}

fn route(c: &Ctx, method: &str, path: &str, auth: Option<&str>, body: &[u8]) -> (u16, Value) {
    let path = path.split('?').next().unwrap_or(path);
    if method == "GET" && path == "/openapi.json" {
        return (200, openapi(&c.model, &c.version));
    }
    let ok = auth.and_then(|a| a.strip_prefix("Bearer ")).is_some_and(|t| same(t.trim(), &c.token));
    if !ok {
        return (401, json!({"error": "a token is needed: Authorization: Bearer TOKEN"}));
    }
    match (method, path) {
        ("GET", "/manifest") => (200, json!({"model_version": c.version, "title": c.model.title, "time_units": c.model.settings.time_units, "initial_time": c.model.settings.initial_time, "final_time": c.model.settings.final_time, "manifest": c.model.manifest})),
        ("POST", "/run") | ("POST", "/batch") => {
            let parsed: Result<Value, _> = if body.is_empty() { Ok(json!({})) } else { serde_json::from_slice(body) };
            let Ok(v) = parsed else { return (400, json!({"error": "the body is not valid JSON"})) };
            let r = if path == "/run" { run_one(c, &v) } else { batch(c, &v) };
            match r {
                Ok(v) => (200, v),
                Err((s, e)) => (s, json!({"error": e})),
            }
        }
        (_, "/manifest") | (_, "/run") | (_, "/batch") | (_, "/openapi.json") => (405, json!({"error": "method not allowed"})),
        _ => (404, json!({"error": "no such path; see /openapi.json"})),
    }
}

fn reason(s: u16) -> &'static str {
    match s {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        422 => "Unprocessable Entity",
        _ => "Internal Server Error",
    }
}

fn handle(c: &Ctx, mut stream: TcpStream) {
    let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();
    let started = std::time::Instant::now();
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut first = String::new();
    if reader.read_line(&mut first).unwrap_or(0) == 0 {
        return;
    }
    let (mut method, mut path) = (String::new(), String::new());
    let mut parts = first.split_whitespace();
    if let (Some(a), Some(b)) = (parts.next(), parts.next()) {
        method = a.to_string();
        path = b.to_string();
    }
    let (mut auth, mut len) = (None, 0usize);
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 || line.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            match k.trim().to_lowercase().as_str() {
                "authorization" => auth = Some(v.trim().to_string()),
                "content-length" => len = v.trim().parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    let (status, body_json, request_text) = if len > 1_000_000 {
        (413, json!({"error": "the body is larger than 1 MB"}), String::new())
    } else {
        let mut body = vec![0u8; len];
        let _ = reader.read_exact(&mut body);
        let (s, v) = route(c, &method, &path, auth.as_deref(), &body);
        (s, v, String::from_utf8_lossy(&body).chars().take(2000).collect())
    };
    let text = body_json.to_string();
    let _ = write!(stream, "HTTP/1.1 {status} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}", reason(status), text.len());
    let _ = stream.flush();
    let line = json!({
        "unix_time": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        "peer": peer, "method": method, "path": path, "status": status, "model_version": c.version,
        "request": request_text, "ms": started.elapsed().as_millis() as u64,
    })
    .to_string();
    if let Some(f) = c.log.lock().unwrap().as_mut() {
        let _ = writeln!(f, "{line}");
    }
    if c.to_stderr {
        eprintln!("{line}");
    }
}

/// Start the server on a thread.
pub fn start(model: Model, cfg: Config) -> Result<Server, String> {
    if cfg.token.trim().is_empty() {
        return Err("a token is needed".into());
    }
    let listener = TcpListener::bind((cfg.bind.as_str(), cfg.port)).map_err(|e| format!("cannot listen on {}:{}: {e}", cfg.bind, cfg.port))?;
    let addr = listener.local_addr().map_err(|e| e.to_string())?;
    let log = match &cfg.log {
        Some(p) => Some(std::fs::OpenOptions::new().create(true).append(true).open(p).map_err(|e| format!("{p}: {e}"))?),
        None => None,
    };
    let ctx = Arc::new(Ctx { version: version_of(&model), model, token: cfg.token.clone(), to_stderr: log.is_none(), log: Mutex::new(log) });
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = stop.clone();
    let handle = std::thread::spawn(move || {
        for conn in listener.incoming() {
            if flag.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            if let Ok(s) = conn {
                let c = ctx.clone();
                std::thread::spawn(move || handle(&c, s));
            }
        }
    });
    Ok(Server { addr, stop, handle: Some(handle) })
}

/// A random token for a server started without one.
pub fn random_token() -> String {
    let mut h = std::collections::hash_map::RandomState::new();
    use std::hash::{BuildHasher, Hasher};
    let mut s = String::new();
    for i in 0..4u64 {
        let mut x = h.build_hasher();
        x.write_u64(i ^ std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0));
        s.push_str(&format!("{:016x}", x.finish()));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use dynamist_core::ops::{self, Op};

    fn model() -> Model {
        let mut m = Model::default();
        m.title = "Tank".into();
        m.settings.final_time = 20.0;
        m.settings.time_step = 0.25;
        m.settings.time_units = "Day".into();
        for j in [
            r#"{"op":"add_var","name":"Tank","kind":"stock","equation":"100","units":"widget"}"#,
            r#"{"op":"add_var","name":"Tau","kind":"constant","equation":"5","units":"Day"}"#,
            r#"{"op":"add_var","name":"Drain","kind":"flow","equation":"Tank / Tau","units":"widget/Day","from":"Tank"}"#,
            r#"{"op":"set_manifest","manifest":{"levers":[{"name":"Tau","min":2,"max":10,"units":"Day"}],"outputs":[{"variable":"Tank","units":"widget"}],"scenarios":[{"name":"Fast","values":{"Tau":3}}],"validity":[{"statement":"short runs","horizon":10}]}}"#,
        ] {
            ops::apply(&mut m, &serde_json::from_str::<Op>(j).unwrap()).unwrap();
        }
        m
    }

    /// One request over a real socket on 127.0.0.1 (the test process only).
    fn call(addr: SocketAddr, method: &str, path: &str, token: Option<&str>, body: &str) -> (u16, Value) {
        let mut s = TcpStream::connect(addr).unwrap();
        let auth = token.map_or(String::new(), |t| format!("Authorization: Bearer {t}\r\n"));
        write!(s, "{method} {path} HTTP/1.1\r\nHost: x\r\n{auth}Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
        let mut text = String::new();
        s.read_to_string(&mut text).unwrap();
        let status: u16 = text.split_whitespace().nth(1).unwrap().parse().unwrap();
        let json = text.split("\r\n\r\n").nth(1).map(|b| serde_json::from_str(b).unwrap_or(Value::Null)).unwrap_or(Value::Null);
        (status, json)
    }

    #[test]
    fn the_service_runs_scenarios_refuses_bad_names_and_ranges_labels_beyond_the_horizon_and_logs_every_call() {
        let log = std::env::temp_dir().join(format!("dyn-serve-{}.log", std::process::id()));
        let _ = std::fs::remove_file(&log);
        let m = model();
        let version = version_of(&m);
        let srv = start(m, Config { bind: "127.0.0.1".into(), port: 0, token: "secret".into(), log: Some(log.to_string_lossy().to_string()) }).unwrap();
        let a = srv.addr;
        // authentication
        assert_eq!(call(a, "GET", "/manifest", None, "").0, 401);
        assert_eq!(call(a, "GET", "/manifest", Some("wrong"), "").0, 401);
        let (s, v) = call(a, "GET", "/manifest", Some("secret"), "");
        assert_eq!((s, v["model_version"].as_str()), (200, Some(version.as_str())));
        assert_eq!(v["manifest"]["levers"][0]["name"], "Tau");
        // the description needs no token and names the levers and the routes
        let (s, v) = call(a, "GET", "/openapi.json", None, "");
        assert!(s == 200 && v["openapi"] == "3.0.3" && v["paths"]["/run"].is_object() && v["x-levers"][0]["maximum"] == 10.0, "{v}");
        // a run: read at the horizon (day 10), the end of the run (day 20) comes under beyond_horizon
        let (s, v) = call(a, "POST", "/run", Some("secret"), r#"{"scenario":"Fast"}"#);
        assert_eq!(s, 200, "{v}");
        assert_eq!(v["read_at"], 10.0);
        let at10 = v["outputs"][0]["value"].as_f64().unwrap();
        let at20 = v["beyond_horizon"]["outputs"][0]["value"].as_f64().unwrap();
        assert!(at10 > at20 && at20 > 0.0 && v["beyond_horizon"]["label"].as_str().unwrap().contains("not vouched for"), "{v}");
        // lever values: the same run by hand, and the base case
        let (_, w) = call(a, "POST", "/run", Some("secret"), r#"{"values":{"tau":3}}"#);
        assert_eq!(w["outputs"][0]["value"], v["outputs"][0]["value"]);
        let (_, base) = call(a, "POST", "/run", Some("secret"), "{}");
        assert!(base["outputs"][0]["value"].as_f64().unwrap() > at10);
        // refused: not a lever, outside the range, no such scenario, an unknown field, not JSON
        assert_eq!(call(a, "POST", "/run", Some("secret"), r#"{"values":{"Tank":50}}"#).0, 400);
        let (s, v) = call(a, "POST", "/run", Some("secret"), r#"{"values":{"Tau":50}}"#);
        assert!(s == 422 && v["error"].as_str().unwrap().contains("2 to 10"), "{v}");
        assert_eq!(call(a, "POST", "/run", Some("secret"), r#"{"scenario":"Nope"}"#).0, 404);
        assert_eq!(call(a, "POST", "/run", Some("secret"), r#"{"colour":1}"#).0, 400);
        assert_eq!(call(a, "POST", "/run", Some("secret"), "not json").0, 400);
        // a batch: each run answered or refused on its own
        let (s, v) = call(a, "POST", "/batch", Some("secret"), r#"{"runs":[{"scenario":"Fast"},{"values":{"Tau":99}}]}"#);
        assert_eq!(s, 200);
        assert_eq!((v["results"][0]["status"].as_u64(), v["results"][1]["status"].as_u64()), (Some(200), Some(422)));
        // the series option labels where the horizon is
        let (_, v) = call(a, "POST", "/run", Some("secret"), r#"{"series":true}"#);
        assert_eq!(v["series"]["beyond_horizon_from"], 10.0);
        srv.stop();
        let lines: Vec<Value> = std::fs::read_to_string(&log).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        assert!(lines.len() >= 14, "{}", lines.len());
        assert!(lines.iter().all(|l| l["model_version"] == version.as_str() && l["status"].is_u64() && l["path"].is_string()));
        assert!(lines.iter().any(|l| l["status"] == 401) && lines.iter().any(|l| l["status"] == 422));
    }

    #[test]
    fn it_listens_on_localhost_by_default_and_needs_a_token() {
        assert!(start(model(), Config { bind: "127.0.0.1".into(), port: 0, token: " ".into(), log: None }).is_err());
        assert_eq!(random_token().len(), 64);
        assert_ne!(random_token(), random_token());
        assert!(same("abc", "abc") && !same("abc", "abd") && !same("abc", "ab"));
    }
}
