use wit_bindgen::generate;

generate!({
    path: "wit",
    world: "demo",
    with: {
        "wasi:keyvalue/store@0.2.0-draft": generate,
    }
});

use wasi::keyvalue::store;

struct Component;

impl Guest for Component {
    fn run(op: String, bucket: String, key: String, value: Option<String>) -> String {
        let b = match store::open(&bucket) {
            Ok(b) => b,
            Err(e) => return format!("error opening bucket: {e:?}"),
        };

        match op.as_str() {
            "set" => {
                let val = value.unwrap_or_default();
                match b.set(&key, val.as_bytes()) {
                    Ok(()) => "ok".to_string(),
                    Err(e) => format!("error: {e:?}"),
                }
            }
            "get" => match b.get(&key) {
                Ok(Some(bytes)) => String::from_utf8_lossy(&bytes).into_owned(),
                Ok(None) => "(not found)".to_string(),
                Err(e) => format!("error: {e:?}"),
            },
            "delete" => match b.delete(&key) {
                Ok(()) => "ok".to_string(),
                Err(e) => format!("error: {e:?}"),
            },
            "exists" => match b.exists(&key) {
                Ok(true) => "true".to_string(),
                Ok(false) => "false".to_string(),
                Err(e) => format!("error: {e:?}"),
            },
            "list" => match b.list_keys(None) {
                Ok(resp) => resp.keys.join(", "),
                Err(e) => format!("error: {e:?}"),
            },
            other => format!("unknown op: {other}"),
        }
    }
}

export!(Component);
