// Copyright 2006-2026 Crytek GmbH / Crytek Group. All rights reserved.

use std::path::Path;

/// Defines whether a filter rule applies to an XML element tag or an attribute name
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterType {
    ElementName,
    AttributeName,
}

/// A single filter rule matching a wildcard pattern
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterElement {
    pub filter_type: FilterType,
    pub accept: bool,
    pub wildcards: String,
}

/// Filter configuration loaded from `xmlfilter.txt` used to strip or convert XML elements/attributes
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct XmlFilter {
    pub filters: Vec<FilterElement>,
    pub table_filemasks: Vec<String>,
}

impl XmlFilter {
    /// Loads filter rules from a configuration file on disk
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::load_from_str(&content))
    }

    /// Parses configuration text format:
    /// - `e - ElementName` (strip element)
    /// - `e + ElementName` (preserve element)
    /// - `a - AttributeName` (strip attribute)
    /// - `a + AttributeName` (preserve attribute)
    /// - `f table *filename.xml` (convert Excel XML to CryEngine Table XML)
    pub fn load_from_str(content: &str) -> Self {
        let mut filter = XmlFilter::default();

        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                continue;
            }

            let first_char = match line.chars().next() {
                Some(c) => c.to_ascii_lowercase(),
                None => continue,
            };
            let remainder = line[1..].trim();

            match first_char {
                'f' => {
                    if let Some(mask) = remainder.strip_prefix("table") {
                        let normalized_mask = mask.trim().replace('/', "\\");
                        if !normalized_mask.is_empty() {
                            filter.table_filemasks.push(normalized_mask);
                        }
                    }
                }
                'a' | 'e' => {
                    let filter_type = if first_char == 'a' {
                        FilterType::AttributeName
                    } else {
                        FilterType::ElementName
                    };

                    if remainder.starts_with('+') || remainder.starts_with('-') {
                        let accept = remainder.starts_with('+');
                        let wildcards = remainder[1..].trim().to_string();
                        if !wildcards.is_empty() {
                            filter.filters.push(FilterElement {
                                filter_type,
                                accept,
                                wildcards,
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        filter
    }

    /// Evaluates whether an element or attribute name passes the filter chain
    pub fn is_accepted(&self, filter_type: FilterType, name: &str) -> bool {
        if self.filters.is_empty() {
            return true;
        }

        for rule in &self.filters {
            if rule.filter_type == filter_type
                && matches_wildcards_ignore_case(name, &rule.wildcards)
            {
                return rule.accept;
            }
        }
        true
    }

    /// Checks if a given XML filename matches an Excel-to-Table conversion filemask
    pub fn matches_table_filemask(&self, filename: &str) -> bool {
        let normalized = filename.replace('/', "\\");
        self.table_filemasks
            .iter()
            .any(|mask| matches_wildcards_ignore_case(&normalized, mask))
    }
}

/// Case-insensitive wildcard pattern matching supporting `*` and `?`
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
