pub fn form_photoshop_data_block(settings_str: &str) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(b"8BIM");
    data.extend_from_slice(&[0x04, 0x04]);
    data.extend_from_slice(&[0x00, 0x00]);

    let irb_size_pos = data.len();
    data.extend_from_slice(&[0, 0, 0, 0]);

    let irb_data_start = data.len();
    data.extend_from_slice(&[0x1C, 0x02, 0x00, 0x02, 0x00, 0x02]);
    data.extend_from_slice(&[0x1C, 0x02, 0x28]);

    let caption_size_pos = data.len();
    data.extend_from_slice(&[0, 0]);

    let caption_start_pos = data.len();
    data.extend_from_slice(settings_str.as_bytes());

    let caption_size = (data.len() - caption_start_pos) as u16;
    data[caption_size_pos..caption_size_pos + 2].copy_from_slice(&caption_size.to_be_bytes());

    let irb_size = (data.len() - irb_data_start) as u32;
    data[irb_size_pos..irb_size_pos + 4].copy_from_slice(&irb_size.to_be_bytes());

    if data.len() % 2 != 0 {
        data.push(0);
    }
    data
}
