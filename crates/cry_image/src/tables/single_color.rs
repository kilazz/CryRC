#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SingleLookupEntry {
    pub c0: u8,
    pub c1: u8,
}

pub const SC_LOOKUP_5_3: [SingleLookupEntry; 256] = {
    let mut lut = [SingleLookupEntry { c0: 0, c1: 0 }; 256];
    let mut i = 0;
    while i < 256 {
        let v = (i as u32 * 31 + 127) / 255;
        let c = ((v << 3) | (v >> 2)) as u8;
        lut[i] = SingleLookupEntry { c0: c, c1: c };
        i += 1;
    }
    lut
};

pub const SC_LOOKUP_6_3: [SingleLookupEntry; 256] = {
    let mut lut = [SingleLookupEntry { c0: 0, c1: 0 }; 256];
    let mut i = 0;
    while i < 256 {
        let v = (i as u32 * 63 + 127) / 255;
        let c = ((v << 2) | (v >> 4)) as u8;
        lut[i] = SingleLookupEntry { c0: c, c1: c };
        i += 1;
    }
    lut
};

pub const SC_LOOKUP_5_4: [SingleLookupEntry; 256] = SC_LOOKUP_5_3;
pub const SC_LOOKUP_6_4: [SingleLookupEntry; 256] = SC_LOOKUP_6_3;
