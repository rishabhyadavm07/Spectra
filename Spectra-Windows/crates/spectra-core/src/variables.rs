use std::collections::HashMap;

/// Case-insensitive variable lookup: keys are matched against a lowercased
/// index so `{{Base_url}}` resolves `base_url`, `BASE_URL`, etc. This exists
/// because case mismatches between how a variable was typed when created and
/// how it's referenced in a request are a common, confusing source of silent
/// "variable not substituted" failures.
struct VarIndex<'a> {
    by_lower: HashMap<String, &'a String>,
}

impl<'a> VarIndex<'a> {
    fn build(vars: &'a HashMap<String, String>) -> Self {
        let mut by_lower = HashMap::with_capacity(vars.len());
        for (k, v) in vars {
            by_lower.insert(k.to_lowercase(), v);
        }
        Self { by_lower }
    }

    fn get(&self, key: &str) -> Option<&'a String> {
        self.by_lower.get(&key.to_lowercase()).copied()
    }
}

/// Replace `{{key}}` occurrences using the given scope, precomputed as a single
/// merged map (later entries win — caller controls precedence order).
/// Lookup is case-insensitive (see `VarIndex`).
pub fn resolve_string(input: &str, vars: &HashMap<String, String>) -> String {
    let index = VarIndex::build(vars);
    resolve_with_index(input, &index)
}

fn resolve_with_index(input: &str, index: &VarIndex) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            if let Some(end) = input[i + 2..].find("}}") {
                let key = input[i + 2..i + 2 + end].trim();
                if let Some(val) = index.get(key) {
                    out.push_str(val);
                } else {
                    out.push_str(&input[i..i + 4 + end]);
                }
                i += 4 + end;
                continue;
            }
        }
        // advance by one char (not byte) to stay UTF-8 safe
        let ch = input[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Scans `input` for `{{key}}` references that have no match in `vars`
/// (case-insensitive), returning the distinct unresolved names in order of
/// first appearance. Used to surface a clear "unresolved variable" error
/// instead of letting an unsubstituted `{{...}}` reach the HTTP layer and
/// fail as an opaque network/URL-parse error.
pub fn find_unresolved(input: &str, vars: &HashMap<String, String>) -> Vec<String> {
    let index = VarIndex::build(vars);
    let mut missing = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            if let Some(end) = input[i + 2..].find("}}") {
                let key = input[i + 2..i + 2 + end].trim().to_string();
                if index.get(&key).is_none() && !missing.contains(&key) {
                    missing.push(key);
                }
                i += 4 + end;
                continue;
            }
        }
        let ch = input[i..].chars().next().unwrap();
        i += ch.len_utf8();
    }
    missing
}

/// Merge variable scopes in precedence order: lowest priority first.
/// PRD Section 8 order (highest to lowest): Runtime > Secret > Request > Folder
/// > Environment > Workspace > Global. We only implement Global/Workspace/
/// Environment/Runtime at this stage; folder/request/secret scopes are future.
pub fn merge_scopes(scopes: Vec<&HashMap<String, String>>) -> HashMap<String, String> {
    let mut merged = HashMap::new();
    for scope in scopes {
        for (k, v) in scope {
            merged.insert(k.clone(), v.clone());
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_known_vars() {
        let mut vars = HashMap::new();
        vars.insert("base_url".to_string(), "https://api.example.com".to_string());
        assert_eq!(
            resolve_string("{{base_url}}/users", &vars),
            "https://api.example.com/users"
        );
    }

    #[test]
    fn leaves_unknown_vars_untouched() {
        let vars = HashMap::new();
        assert_eq!(resolve_string("{{missing}}/x", &vars), "{{missing}}/x");
    }

    #[test]
    fn resolves_case_insensitively() {
        let mut vars = HashMap::new();
        vars.insert("Base_url".to_string(), "https://api.example.com".to_string());
        assert_eq!(
            resolve_string("{{base_url}}/users", &vars),
            "https://api.example.com/users"
        );
        assert_eq!(
            resolve_string("{{BASE_URL}}/users", &vars),
            "https://api.example.com/users"
        );
    }

    #[test]
    fn find_unresolved_reports_missing_vars_once_each() {
        let mut vars = HashMap::new();
        vars.insert("token".to_string(), "abc".to_string());
        let missing = find_unresolved("{{base_url}}/{{token}}/{{base_url}}", &vars);
        assert_eq!(missing, vec!["base_url".to_string()]);
    }
}
