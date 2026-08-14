//! Funções utilitárias do proxy que não são rota nem regra de negócio.
//!
//! ===================== Turbo-stream decoder =====================
//! Tudo abaixo (até o fim do arquivo) é o decoder do formato "turbo-stream"
//! (Remix single-fetch) usado por
//! https://web.construcode.com.br/Enterprises.data — a resposta não é JSON
//! puro, é um array "achatado" onde cada elemento pode referenciar outros
//! por índice. Porta 1:1 do decoder Python em docs/construcode_login.py.
//! Ver https://github.com/jacob-ebey/turbo-stream
//! Ponto de entrada público: `turbo_stream_decode`.
//! ==================================================================

use std::collections::HashMap;

use serde_json::{Map, Value};

const HOLE: i64 = -1;
const NULL: i64 = -5;

fn is_sentinel(index: i64) -> bool {
    // NAN, NEG_INF, NEG_ZERO, POS_INF e UNDEF não têm representação útil em
    // serde_json::Value — tratamos todos como null (não aparecem nos dados
    // reais de empreendimentos, só em bordas específicas do formato JS).
    (-7..=-2).contains(&index)
}

fn is_tag(s: &str) -> bool {
    matches!(
        s,
        "D" | "U" | "B" | "R" | "Y" | "S" | "M" | "N" | "P" | "E" | "Z"
    )
}

struct TurboDecoder {
    values: Vec<Value>,
    hydrated: HashMap<i64, Value>,
}

impl TurboDecoder {
    fn new() -> Self {
        Self {
            values: Vec::new(),
            hydrated: HashMap::new(),
        }
    }

    fn unflatten(&mut self, parsed: Value) -> Value {
        if let Some(idx) = parsed.as_i64() {
            return self.hydrate(idx);
        }
        let start = self.values.len() as i64;
        match parsed {
            Value::Array(arr) => self.values.extend(arr),
            other => self.values.push(other),
        }
        self.hydrate(start)
    }

    fn hydrate(&mut self, index: i64) -> Value {
        if is_sentinel(index) || index == NULL {
            return Value::Null;
        }
        if let Some(cached) = self.hydrated.get(&index) {
            return cached.clone();
        }

        let value = self.values[index as usize].clone();

        let result = match &value {
            Value::Array(arr) => match arr.first() {
                Some(Value::String(tag)) if is_tag(tag) => self.hydrate_tagged(tag, arr),
                _ => self.hydrate_plain_array(arr),
            },
            Value::Object(obj) => self.hydrate_indexed_object(obj),
            primitive => primitive.clone(),
        };

        self.hydrated.insert(index, result.clone());
        result
    }

    fn hydrate_tagged(&mut self, tag: &str, arr: &[Value]) -> Value {
        match tag {
            // Date/undefined/RegExp/Symbol: valor literal embutido, não é índice.
            "D" | "U" | "R" | "Y" => arr.get(1).cloned().unwrap_or(Value::Null),
            "B" => match arr.get(1) {
                Some(Value::String(s)) => s
                    .trim_end_matches('n')
                    .parse::<i64>()
                    .map(Value::from)
                    .unwrap_or(Value::Null),
                Some(other) => other.clone(),
                None => Value::Null,
            },
            "S" => Value::Array(
                arr.iter()
                    .skip(1)
                    .map(|item| self.hydrate(item.as_i64().unwrap_or(NULL)))
                    .collect(),
            ),
            "M" => {
                let mut map = Map::new();
                let mut i = 1;
                while i + 1 < arr.len() {
                    let key = self.hydrate(arr[i].as_i64().unwrap_or(NULL));
                    let val = self.hydrate(arr[i + 1].as_i64().unwrap_or(NULL));
                    map.insert(value_as_key(&key), val);
                    i += 2;
                }
                Value::Object(map)
            }
            "N" => {
                let mut map = Map::new();
                if let Some(Value::Object(spec)) = arr.get(1) {
                    for (raw_key, vidx) in spec {
                        let key = self.hydrate(key_index(raw_key));
                        let val = self.hydrate(vidx.as_i64().unwrap_or(NULL));
                        map.insert(value_as_key(&key), val);
                    }
                }
                Value::Object(map)
            }
            "P" => {
                let deferred_id = arr.get(1).and_then(Value::as_i64).unwrap_or(-1);
                promise_marker(deferred_id)
            }
            "E" => {
                let mut map = Map::new();
                map.insert(
                    "__error".to_string(),
                    arr.get(1).cloned().unwrap_or(Value::Null),
                );
                Value::Object(map)
            }
            "Z" => self.hydrate(arr.get(1).and_then(Value::as_i64).unwrap_or(NULL)),
            _ => Value::Array(arr.to_vec()),
        }
    }

    fn hydrate_plain_array(&mut self, arr: &[Value]) -> Value {
        Value::Array(
            arr.iter()
                .map(|n| match n.as_i64() {
                    Some(HOLE) => Value::Null,
                    Some(idx) => self.hydrate(idx),
                    None => Value::Null,
                })
                .collect(),
        )
    }

    fn hydrate_indexed_object(&mut self, obj: &Map<String, Value>) -> Value {
        let mut result = Map::new();
        for (raw_key, vidx) in obj {
            let key = self.hydrate(key_index(raw_key));
            let val = self.hydrate(vidx.as_i64().unwrap_or(NULL));
            result.insert(value_as_key(&key), val);
        }
        Value::Object(result)
    }
}

// Chaves de objeto no formato vêm como "$3" (prefixo + índice de outro
// valor que hydrata pro nome real da propriedade).
fn key_index(raw_key: &str) -> i64 {
    raw_key
        .get(1..)
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(NULL)
}

fn value_as_key(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn promise_marker(deferred_id: i64) -> Value {
    let mut map = Map::new();
    map.insert("__turbo_promise__".to_string(), Value::from(deferred_id));
    Value::Object(map)
}

fn substitute_promises(value: Value, deferred: &HashMap<i64, Value>) -> Value {
    match value {
        Value::Object(obj) if obj.len() == 1 && obj.contains_key("__turbo_promise__") => {
            match obj.get("__turbo_promise__").and_then(Value::as_i64) {
                Some(id) => match deferred.get(&id) {
                    Some(resolved) => substitute_promises(resolved.clone(), deferred),
                    None => Value::Null,
                },
                None => Value::Object(obj),
            }
        }
        Value::Object(obj) => Value::Object(
            obj.into_iter()
                .map(|(k, v)| (k, substitute_promises(v, deferred)))
                .collect(),
        ),
        Value::Array(arr) => {
            Value::Array(arr.into_iter().map(|v| substitute_promises(v, deferred)).collect())
        }
        other => other,
    }
}

/// Decodifica o corpo inteiro da resposta turbo-stream num `serde_json::Value`
/// navegável normalmente (objetos/arrays já resolvidos, sem mais índices).
pub fn turbo_stream_decode(text: &str) -> Value {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());

    let mut decoder = TurboDecoder::new();

    let Some(first_line) = lines.next() else {
        return Value::Null;
    };
    let first_parsed: Value = serde_json::from_str(first_line).unwrap_or(Value::Null);
    let root = decoder.unflatten(first_parsed);

    let mut deferred_results: HashMap<i64, Value> = HashMap::new();
    for line in lines {
        let Some(colon) = line.find(':') else { continue };
        let Some(deferred_id) = line.get(1..colon).and_then(|s| s.parse::<i64>().ok()) else {
            continue;
        };
        let payload: Value = serde_json::from_str(&line[colon + 1..]).unwrap_or(Value::Null);
        let hydrated = decoder.unflatten(payload);
        deferred_results.insert(deferred_id, hydrated);
    }

    substitute_promises(root, &deferred_results)
}
