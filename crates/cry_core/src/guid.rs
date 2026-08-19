use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CryGuid {
    pub data: [u8; 16],
}

impl Default for CryGuid {
    fn default() -> Self {
        Self::null()
    }
}

impl CryGuid {
    pub const fn null() -> Self {
        Self { data: [0u8; 16] }
    }

    pub fn is_null(&self) -> bool {
        self.data.iter().all(|&b| b == 0)
    }

    pub fn create() -> Self {
        let mut data = [0u8; 16];
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let nanos = now.as_nanos();
        let stack_seed = &data as *const _ as usize;

        let mut state = (nanos as u64) ^ (stack_seed as u64);
        for chunk in data.chunks_exact_mut(8) {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            chunk.copy_from_slice(&state.to_le_bytes());
        }

        data[6] = (data[6] & 0x0F) | 0x40;
        data[8] = (data[8] & 0x3F) | 0x80;
        Self { data }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        let clean = s
            .trim()
            .trim_matches(|c| c == '{' || c == '}')
            .replace('-', "");
        if clean.len() != 32 {
            return Err(format!("Invalid CryGUID length: '{}'", s));
        }

        let mut data = [0u8; 16];
        for i in 0..16 {
            data[i] = u8::from_str_radix(&clean[i * 2..i * 2 + 2], 16)
                .map_err(|e| format!("Invalid CryGUID hex: {}", e))?;
        }
        Ok(Self { data })
    }
}

impl fmt::Display for CryGuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{{cc0:02X}{cc1:02X}{cc2:02X}{cc3:02X}-{cc4:02X}{cc5:02X}-{cc6:02X}{cc7:02X}-{cc8:02X}{cc9:02X}-{cc10:02X}{cc11:02X}{cc12:02X}{cc13:02X}{cc14:02X}{cc15:02X}}}",
            cc0 = self.data[0],
            cc1 = self.data[1],
            cc2 = self.data[2],
            cc3 = self.data[3],
            cc4 = self.data[4],
            cc5 = self.data[5],
            cc6 = self.data[6],
            cc7 = self.data[7],
            cc8 = self.data[8],
            cc9 = self.data[9],
            cc10 = self.data[10],
            cc11 = self.data[11],
            cc12 = self.data[12],
            cc13 = self.data[13],
            cc14 = self.data[14],
            cc15 = self.data[15]
        )
    }
}
