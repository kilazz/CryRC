#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct BitStream128(pub u128);

impl BitStream128 {
    #[inline(always)]
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    #[inline(always)]
    pub fn read_bits(&self, offset: u32, count: u32) -> u32 {
        if count == 0 || offset >= 128 {
            return 0;
        }
        let actual_count = count.min(128 - offset);
        let mask = if actual_count >= 128 {
            u128::MAX
        } else {
            (1u128 << actual_count) - 1
        };
        ((self.0 >> offset) & mask) as u32
    }

    #[inline(always)]
    pub fn write_bits(&mut self, offset: u32, count: u32, value: u32) {
        if count == 0 || offset >= 128 {
            return;
        }
        let actual_count = count.min(128 - offset);
        let val_mask = if actual_count >= 128 {
            u128::MAX
        } else {
            (1u128 << actual_count) - 1
        };
        let mask = val_mask << offset;
        self.0 = (self.0 & !mask) | ((value as u128 & val_mask) << offset);
    }

    #[inline(always)]
    pub const fn to_bytes(&self) -> [u8; 16] {
        self.0.to_le_bytes()
    }

    #[inline(always)]
    pub const fn from_bytes(bytes: &[u8; 16]) -> Self {
        Self(u128::from_le_bytes(*bytes))
    }
}
