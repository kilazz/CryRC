#[derive(Debug, Clone)]
pub struct NameRule {
    pub mask: String,
    pub format: String,
}

#[derive(Debug, Clone, Default)]
pub struct NameConverter {
    rules: Vec<NameRule>,
}

impl NameConverter {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn set_rules(&mut self, rules_str: &str) -> Result<(), String> {
        self.rules.clear();
        for pair in rules_str.split(';') {
            let tokens: Vec<&str> = pair.split(',').collect();
            if tokens.len() == 2 {
                self.rules.push(NameRule {
                    mask: tokens[0].trim().to_string(),
                    format: tokens[1].trim().to_string(),
                });
            }
        }
        Ok(())
    }

    pub fn has_rules(&self) -> bool {
        !self.rules.is_empty()
    }

    pub fn convert_name(&self, original_name: &str) -> String {
        for rule in &self.rules {
            if matches_wildcards_ignore_case(original_name, &rule.mask) {
                return rule.format.replace("{0}", original_name);
            }
        }
        original_name.to_string()
    }
}

pub fn matches_wildcards_ignore_case(text: &str, pattern: &str) -> bool {
    let t: Vec<char> = text.to_ascii_lowercase().chars().collect();
    let p: Vec<char> = pattern.to_ascii_lowercase().chars().collect();

    let mut t_idx = 0;
    let mut p_idx = 0;
    let mut star_idx = None;
    let mut match_idx = 0;

    while t_idx < t.len() {
        if p_idx < p.len() && (p[p_idx] == '?' || p[p_idx] == t[t_idx]) {
            p_idx += 1;
            t_idx += 1;
        } else if p_idx < p.len() && p[p_idx] == '*' {
            star_idx = Some(p_idx);
            match_idx = t_idx;
            p_idx += 1;
        } else if let Some(star) = star_idx {
            p_idx = star + 1;
            match_idx += 1;
            t_idx = match_idx;
        } else {
            return false;
        }
    }
    while p_idx < p.len() && p[p_idx] == '*' {
        p_idx += 1;
    }
    p_idx == p.len()
}
