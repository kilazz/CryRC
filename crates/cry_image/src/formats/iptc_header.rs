use std::collections::HashMap;

pub const IPTC_TAG_MARKER: u8 = 0x1C;
pub const IPTC_RECORD_APP_DATASET: u8 = 0x02;
pub const FIELD_SPECIAL_INSTRUCTIONS: u8 = 0x28;

#[derive(Debug, Clone, Default)]
pub struct IptcHeader {
    pub fields: HashMap<u8, Vec<Vec<u8>>>,
}

impl IptcHeader {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    pub fn parse(&mut self, buffer: &[u8]) {
        self.fields.clear();
        let mut pos = 0;

        while pos + 5 < buffer.len() {
            if buffer[pos] != IPTC_TAG_MARKER {
                return;
            }
            pos += 1;
            let record_type = buffer[pos];
            pos += 1;
            let dataset_id = buffer[pos];
            pos += 1;
            let field_len = ((buffer[pos] as usize) << 8) | (buffer[pos + 1] as usize);
            pos += 2;

            if pos + field_len > buffer.len() {
                return;
            }

            if record_type == IPTC_RECORD_APP_DATASET {
                let data = buffer[pos..pos + field_len].to_vec();
                self.fields.entry(dataset_id).or_default().push(data);
            }
            pos += field_len;
        }
    }

    pub fn get_combined_fields(&self, field_id: u8, separator: &str) -> String {
        if let Some(list) = self.fields.get(&field_id) {
            let strings: Vec<String> = list
                .iter()
                .map(|b| String::from_utf8_lossy(b).trim().to_string())
                .collect();
            strings.join(separator)
        } else {
            String::new()
        }
    }

    pub fn build_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        for (&dataset_id, entries) in &self.fields {
            for entry in entries {
                buffer.push(IPTC_TAG_MARKER);
                buffer.push(IPTC_RECORD_APP_DATASET);
                buffer.push(dataset_id);
                buffer.push(((entry.len() >> 8) & 0xFF) as u8);
                buffer.push((entry.len() & 0xFF) as u8);
                buffer.extend_from_slice(entry);
            }
        }
        buffer
    }
}
