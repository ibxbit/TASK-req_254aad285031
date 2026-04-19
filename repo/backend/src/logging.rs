// Lightweight structured logging that emits one JSON object per event to
// stderr via Rocket's default logger. No external sinks, offline-safe.
//
// Callers use `event!(level, event=..., ...)` to emit a record. Sensitive
// fields are masked with `mask_value` before serialization — never log raw
// identifiers/tokens.

use serde_json::{json, Value};

pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    fn as_str(&self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

/// Emit a structured log line. `event` is the short machine name (e.g.
/// "auth.login.failed"). `fields` is an already-constructed serde_json
/// Object. Use `redact_sensitive_keys` on any user-supplied payload before
/// passing it in.
pub fn emit(level: Level, event: &str, mut fields: Value) {
    let ts = chrono::Utc::now().to_rfc3339();
    if let Some(obj) = fields.as_object_mut() {
        obj.insert("ts".into(), json!(ts));
        obj.insert("level".into(), json!(level.as_str()));
        obj.insert("event".into(), json!(event));
    } else {
        fields = json!({
            "ts": ts,
            "level": level.as_str(),
            "event": event,
        });
    }
    // stderr: does not interfere with HTTP response bodies or stdout readers.
    eprintln!("{}", fields);
}

/// Replace values for well-known sensitive keys with "[redacted]". Operates
/// recursively on nested objects / arrays. Keys matched case-insensitively.
pub fn redact_sensitive_keys(mut v: Value) -> Value {
    fn is_sensitive(key: &str) -> bool {
        let k = key.to_ascii_lowercase();
        k.contains("password")
            || k.contains("token")
            || k == "authorization"
            || k == "sensitive_id"
            || k == "ssn"
            || k.contains("secret")
            || k.contains("api_key")
    }
    match &mut v {
        Value::Object(m) => {
            for (k, val) in m.iter_mut() {
                if is_sensitive(k) {
                    *val = Value::String("[redacted]".into());
                } else {
                    let taken = std::mem::take(val);
                    *val = redact_sensitive_keys(taken);
                }
            }
        }
        Value::Array(a) => {
            for item in a.iter_mut() {
                let taken = std::mem::take(item);
                *item = redact_sensitive_keys(taken);
            }
        }
        _ => {}
    }
    v
}

/// Convenience helpers for the most common security/policy events. Keeps
/// event names stable so operators can grep and graph them.
pub fn auth_failure(username: &str, reason: &str) {
    emit(
        Level::Warn,
        "auth.login.failed",
        json!({
            "username": username,
            "reason": reason,
        }),
    );
}

pub fn permission_denied(user_id: &str, role: &str, route: &str) {
    emit(
        Level::Warn,
        "authz.denied",
        json!({
            "user_id": user_id,
            "role": role,
            "route": route,
        }),
    );
}

pub fn validation_failed(route: &str, reason: &str) {
    emit(
        Level::Warn,
        "validation.failed",
        json!({
            "route": route,
            "reason": reason,
        }),
    );
}

pub fn audit_write_failed(entity: &str, action: &str, err: &str) {
    emit(
        Level::Error,
        "audit.write.failed",
        json!({
            "entity": entity,
            "action": action,
            "error": err,
        }),
    );
}

pub fn account_locked(user_id: &str) {
    emit(
        Level::Warn,
        "auth.account.locked",
        json!({
            "user_id": user_id,
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_password_field() {
        let v = json!({"username":"u","password":"p","token":"t"});
        let r = redact_sensitive_keys(v);
        assert_eq!(r["username"], "u");
        assert_eq!(r["password"], "[redacted]");
        assert_eq!(r["token"], "[redacted]");
    }

    #[test]
    fn redact_nested() {
        let v = json!({"outer": {"ssn": "123-45-6789", "ok": "x"}});
        let r = redact_sensitive_keys(v);
        assert_eq!(r["outer"]["ssn"], "[redacted]");
        assert_eq!(r["outer"]["ok"], "x");
    }
}
