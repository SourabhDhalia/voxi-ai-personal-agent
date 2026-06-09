impl AgentCore {
    // Grounding helper methods
}

fn shopping_state_path(session_workdir: &Path, session_id: &str) -> PathBuf {
    let safe_session = session_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    session_workdir
        .join("state")
        .join("shopping")
        .join(format!("{}.json", safe_session))
}

fn parse_numbered_selection(input: &str) -> Option<usize> {
    let lower = input.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return None;
    }
    let token = lower
        .split_whitespace()
        .find(|part| part.chars().next().is_some_and(|ch| ch.is_ascii_digit()))?;
    let digits = token
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits.parse::<usize>().ok().filter(|value| *value > 0)
}

fn resolve_selection_index(
    session_workdir: &Path,
    session_id: &str,
    prompt: &str,
) -> Option<usize> {
    let lower = prompt.trim().to_lowercase();

    // Check for explicit digit selection first
    if let Some(num) = parse_numbered_selection(prompt) {
        return Some(num);
    }

    // Check ordinals
    if lower.contains("first") || lower.contains("1st") {
        return Some(1);
    }
    if lower.contains("second") || lower.contains("2nd") {
        return Some(2);
    }
    if lower.contains("third") || lower.contains("3rd") {
        return Some(3);
    }

    // Check for cheapest selection
    if lower.contains("cheapest") || lower.contains("lowest price") || lower.contains("minimum price") {
        let path = shopping_state_path(session_workdir, session_id);
        let state_text = std::fs::read_to_string(path).ok()?;
        let state: Value = serde_json::from_str(&state_text).ok()?;
        let options = state.get("options").and_then(Value::as_array)?;

        let mut min_price = None;
        let mut min_number = None;
        for opt in options {
            let number = opt.get("number").and_then(Value::as_u64)?;
            let raw = opt.get("raw")?;

            // Extract numeric price
            let price = raw.get("price")
                .and_then(|p| {
                    if p.is_number() {
                        p.as_f64()
                    } else {
                        p.get("offerPrice")
                            .and_then(Value::as_f64)
                            .or_else(|| p.get("mrp").and_then(Value::as_f64))
                    }
                })
                .or_else(|| raw.get("sellingPrice").and_then(Value::as_f64))
                .or_else(|| raw.get("mrp").and_then(Value::as_f64));

            if let Some(p) = price {
                if min_price.is_none() || Some(p) < min_price {
                    min_price = Some(p);
                    min_number = Some(number as usize);
                }
            }
        }
        return min_number;
    }

    None
}

fn shopping_selection_context(
    session_workdir: &Path,
    session_id: &str,
    prompt: &str,
) -> Option<String> {
    let path = shopping_state_path(session_workdir, session_id);
    let mut state: Value = if let Ok(state_text) = std::fs::read_to_string(&path) {
        serde_json::from_str(&state_text).unwrap_or(json!({}))
    } else {
        json!({})
    };

    let selected_number = if let Some(num) = resolve_selection_index(session_workdir, session_id, prompt) {
        state["selected_number"] = json!(num);
        if let Ok(text) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&path, text);
        }
        num
    } else {
        state.get("selected_number").and_then(Value::as_u64).map(|n| n as usize)?
    };

    let options = state.get("options").and_then(Value::as_array)?;
    let selected = options.iter().find(|option| {
        option
            .get("number")
            .and_then(Value::as_u64)
            .map(|value| value as usize == selected_number)
            .unwrap_or(false)
    })?;

    Some(format!(
        "## Shopping Selection Context\nThe user selected option {} from the latest shopping results. Use the preserved provider identifiers from this JSON instead of long-term memory or display-only IDs:\n{}",
        selected_number,
        selected
    ))
}

fn store_shopping_options_from_search_result(
    session_workdir: &Path,
    session_id: &str,
    tool_name: &str,
    result: &Value,
) {
    if !tool_name.to_ascii_lowercase().contains("search") {
        return;
    }
    let provider = provider_from_mcp_tool_name(tool_name);
    let mut raw_options = Vec::new();
    collect_shopping_option_objects(result, &mut raw_options);
    if raw_options.is_empty() {
        return;
    }

    let options = raw_options
        .into_iter()
        .take(20)
        .enumerate()
        .map(|(idx, raw)| {
            json!({
                "number": idx + 1,
                "provider": provider,
                "source_tool": tool_name,
                "display": shopping_option_display(&provider, &raw),
                "identifier_hints": shopping_identifier_hints(&raw),
                "raw": raw,
            })
        })
        .collect::<Vec<_>>();

    let state = json!({
        "session_id": session_id,
        "provider": provider,
        "source_tool": tool_name,
        "options": options,
    });
    let path = shopping_state_path(session_workdir, session_id);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(&state) {
        let _ = std::fs::write(path, text);
    }
}

fn provider_from_mcp_tool_name(tool_name: &str) -> String {
    let without_prefix = tool_name.strip_prefix("mcp_").unwrap_or(tool_name);
    if without_prefix.starts_with("swiggy_instamart_") {
        return "swiggy-instamart".to_string();
    }
    if without_prefix.starts_with("swiggy_food_") {
        return "swiggy-food".to_string();
    }
    if without_prefix.starts_with("swiggy_dineout_") {
        return "swiggy-dineout".to_string();
    }
    if without_prefix.starts_with("zepto_") {
        return "zepto".to_string();
    }
    if let Some((provider, _)) = without_prefix.split_once('_') {
        return provider.replace('_', "-");
    }
    without_prefix.replace('_', "-")
}

fn collect_shopping_option_objects(value: &Value, out: &mut Vec<Value>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_shopping_option_objects(item, out);
            }
        }
        Value::Object(map) => {
            if looks_like_shopping_option(map) {
                out.push(Value::Object(map.clone()));
            }
            for value in map.values() {
                collect_shopping_option_objects(value, out);
            }
        }
        _ => {}
    }
}

fn looks_like_shopping_option(map: &serde_json::Map<String, Value>) -> bool {
    let has_label = ["name", "title", "displayName", "productName", "brand", "description"]
        .iter()
        .any(|key| map.get(*key).is_some());
    let has_commerce_field = map.keys().any(|key| {
        let lower = key.to_ascii_lowercase();
        lower.contains("id")
            || lower.contains("price")
            || lower.contains("mrp")
            || lower.contains("stock")
            || lower.contains("available")
    });
    has_label && has_commerce_field
}

fn shopping_option_display(provider: &str, raw: &Value) -> String {
    let name = first_string_field(
        raw,
        &["name", "title", "displayName", "productName", "description"],
    )
    .unwrap_or_else(|| "item".to_string());
    let size = first_string_field(raw, &["quantity", "unit", "packSize", "weight"])
        .unwrap_or_else(|| "-".to_string());
    let price = first_value_text(raw, &["price", "finalPrice", "salePrice", "mrp"])
        .unwrap_or_else(|| "-".to_string());
    let availability = first_value_text(raw, &["availability", "available", "inStock", "in_stock"])
        .unwrap_or_else(|| "-".to_string());
    format!(
        "{} - {}, size {}, price {}, availability {}",
        provider, name, size, price, availability
    )
}

fn shopping_identifier_hints(raw: &Value) -> Value {
    let mut map = serde_json::Map::new();
    collect_identifier_hints(raw, &mut map);
    Value::Object(map)
}

fn collect_identifier_hints(value: &Value, out: &mut serde_json::Map<String, Value>) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                let lower = key.to_ascii_lowercase();
                if lower.contains("id")
                    || lower == "spinid"
                    || lower == "spin_id"
                    || lower == "skuid"
                    || lower == "sku_id"
                    || lower == "variantid"
                    || lower == "variant_id"
                {
                    out.insert(key.clone(), value.clone());
                }
                collect_identifier_hints(value, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_identifier_hints(item, out);
            }
        }
        _ => {}
    }
}

fn first_string_field(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str).map(ToString::to_string))
}

fn first_value_text(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value.get(*key).map(|value| {
            value
                .as_str()
                .map(ToString::to_string)
                .unwrap_or_else(|| value.to_string())
        })
    })
}

fn compact_shopping_search_result(
    value: &Value,
    query: &str,
    critical_keys: &std::collections::HashSet<String>,
) -> Value {
    match value {
        Value::Array(arr) => {
            if arr.is_empty() {
                return Value::Array(vec![]);
            }
            if arr[0].is_object() {
                // Check if this array is an MCP content array (which contains {"type": "text", "text": "..."})
                let is_mcp_content = arr[0].get("type").is_some()
                    && (arr[0].get("text").is_some() || arr[0].get("image").is_some());

                if is_mcp_content {
                    return Value::Array(
                        arr.iter()
                            .map(|v| compact_shopping_search_result(v, query, critical_keys))
                            .collect(),
                    );
                }

                // If not MCP content, it is a list of product/item objects. Score and keep top 10.
                let query_lower = query.to_lowercase();
                let query_words: Vec<&str> = query_lower.split_whitespace().collect();

                let mut scored: Vec<(usize, Value)> = arr
                    .iter()
                    .map(|item| {
                        let score = if let Some(obj) = item.as_object() {
                            let mut s = 0usize;
                            for &field in &["name", "title", "brand", "description", "displayName", "brandName"] {
                                if let Some(val_str) = obj.get(field).and_then(|v| v.as_str()) {
                                    let val_lower = val_str.to_lowercase();
                                    for word in &query_words {
                                        if val_lower.contains(word) {
                                            s += 10;
                                        }
                                    }
                                }
                            }
                            s
                        } else {
                            0
                        };
                        (score, item.clone())
                    })
                    .collect();

                scored.sort_by(|a, b| b.0.cmp(&a.0));

                let top_10: Vec<Value> = scored
                    .into_iter()
                    .take(10)
                    .map(|(_, item)| compact_shopping_search_result(&item, query, critical_keys))
                    .collect();

                Value::Array(top_10)
            } else {
                Value::Array(
                    arr.iter()
                        .map(|v| compact_shopping_search_result(v, query, critical_keys))
                        .collect(),
                )
            }
        }
        Value::Object(obj) => {
            // Check if there is a query field in this object to update the search term for nested arrays
            let mut current_query = query.to_string();
            if let Some(Value::String(q)) = obj.get("query") {
                if !q.is_empty() {
                    current_query = q.clone();
                }
            }

            // Check if this is an MCP text content object
            if let (Some(Value::String(mcp_type)), Some(Value::String(text_val))) =
                (obj.get("type"), obj.get("text"))
            {
                if mcp_type == "text" {
                    // Try parsing the text value as JSON
                    if let Ok(parsed_json) = serde_json::from_str::<Value>(text_val) {
                        let compacted_json =
                            compact_shopping_search_result(&parsed_json, &current_query, critical_keys);
                        let compacted_str = serde_json::to_string(&compacted_json)
                            .unwrap_or_else(|_| text_val.clone());
                        let mut new_obj = obj.clone();
                        new_obj.insert("text".to_string(), Value::String(compacted_str));
                        return Value::Object(new_obj);
                    }
                }
            }

            // Normal object pruning
            let mut new_obj = serde_json::Map::new();
            for (k, v) in obj {
                let k_lower = k.to_lowercase();

                // 1. Skip known tracking/analytics/SEO metadata blocks
                if k_lower.contains("tracking")
                    || k_lower.contains("analytics")
                    || k_lower.contains("seo")
                    || k_lower.contains("badge")
                    || k_lower.contains("pixel")
                    || k_lower.contains("clickurl")
                {
                    continue;
                }

                // 2. Keep the key if it matches critical harvested parameter keys
                // We use case-insensitive, alphanumeric-only normalized check
                let norm_k: String = k_lower.chars().filter(|c| c.is_alphanumeric()).collect();
                let matches_critical = critical_keys.iter().any(|ck| {
                    let norm_ck: String = ck.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect();
                    norm_k == norm_ck
                });

                if matches_critical {
                    new_obj.insert(
                        k.clone(),
                        compact_shopping_search_result(v, &current_query, critical_keys),
                    );
                    continue;
                }

                // 3. Keep standard database identifier suffix patterns (e.g. *id, *Id, *ID, *_id, *-id)
                let is_id_suffix = k_lower.ends_with("id")
                    || k_lower.ends_with("_id")
                    || k_lower.ends_with("-id");

                // 4. Keep standard commerce descriptive, financial, and availability keys
                let is_commerce_key = matches!(
                    k.as_str(),
                    "id" | "productId" | "product_id" | "name" | "title" | "price" |
                    "mrp" | "brand" | "quantity" | "unit" | "inStock" | "in_stock" |
                    "available" | "availability" | "discount" | "displayName" |
                    "variations" | "quantityDescription" | "brandName" | "offerPrice" |
                    "isInStockAndAvailable" | "isAvail" | "isPromoted" | "success" |
                    "data" | "products" | "message" | "packSize" | "availableQuantity" |
                    "productVariantId" | "storeProductId" | "cartProductId" | "variantId" |
                    "structuredContent" | "content" | "type" | "text"
                );

                if is_id_suffix || is_commerce_key {
                    new_obj.insert(
                        k.clone(),
                        compact_shopping_search_result(v, &current_query, critical_keys),
                    );
                    continue;
                }

                // 5. Value-based pruning (truncating long strings or replacing image URLs)
                match v {
                    Value::String(s) => {
                        if s.len() > 150 {
                            new_obj.insert(k.clone(), Value::String(format!("{}...", &s[..147])));
                        } else if s.starts_with("http") && (
                            s.ends_with(".png") || s.ends_with(".jpg") || s.ends_with(".jpeg") ||
                            s.ends_with(".webp") || s.ends_with(".gif") || s.ends_with(".svg") || s.len() > 70
                        ) {
                            new_obj.insert(k.clone(), Value::String("[MEDIA_URL]".to_string()));
                        } else {
                            new_obj.insert(k.clone(), Value::String(s.clone()));
                        }
                    }
                    other => {
                        new_obj.insert(
                            k.clone(),
                            compact_shopping_search_result(other, &current_query, critical_keys),
                        );
                    }
                }
            }
            Value::Object(new_obj)
        }
        _ => value.clone(),
    }
}

fn sanitize_for_log(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sanitized = serde_json::Map::new();
            for (k, v) in map {
                let k_lower = k.to_lowercase();
                if k_lower.contains("token") || k_lower.contains("cookie") || k_lower.contains("secret") || k_lower.contains("password") || k_lower.contains("key") || k_lower.contains("auth") {
                    sanitized.insert(k.clone(), Value::String("[REDACTED]".to_string()));
                } else {
                    sanitized.insert(k.clone(), sanitize_for_log(v));
                }
            }
            Value::Object(sanitized)
        }
        Value::Array(arr) => {
            Value::Array(arr.iter().map(sanitize_for_log).collect())
        }
        _ => value.clone(),
    }
}
