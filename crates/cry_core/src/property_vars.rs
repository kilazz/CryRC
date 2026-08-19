use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct PropertyVars {
    properties: HashMap<String, String>,
}

impl PropertyVars {
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
        }
    }

    pub fn set_property(&mut self, name: &str, value: &str) {
        self.properties
            .insert(name.to_ascii_lowercase(), value.to_string());
    }

    pub fn get_property(&self, name: &str) -> Option<&str> {
        self.properties
            .get(&name.to_ascii_lowercase())
            .map(|s| s.as_str())
    }

    pub fn expand_properties(&self, input: &mut String) -> Result<(), String> {
        let original = input.clone();
        let mut iterations = 0;

        loop {
            let start = match input.find("${") {
                Some(s) => s,
                None => return Ok(()),
            };

            let end = match input[start + 2..].find('}') {
                Some(e) => start + 2 + e,
                None => return Ok(()),
            };

            let prop_name = &input[start + 2..end].to_ascii_lowercase();
            let replacement = self.properties.get(prop_name).ok_or_else(|| {
                format!(
                    "Unknown property '${{{}}}' in string '{}'",
                    prop_name, original
                )
            })?;

            input.replace_range(start..=end, replacement);
            iterations += 1;
            if iterations > 100 {
                return Err(format!(
                    "Infinite recursion detected in property variable '${{{}}}'",
                    prop_name
                ));
            }
        }
    }
}
