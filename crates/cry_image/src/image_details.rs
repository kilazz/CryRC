use byteorder::{ByteOrder, LittleEndian};
use cry_asset::AssetDetail;
use std::fs;
use std::path::Path;

pub struct ImageDetails;

impl ImageDetails {
    pub fn collect_dds_details(
        dds_path: &Path,
        details: &mut Vec<AssetDetail>,
    ) -> Result<(), String> {
        let raw = fs::read(dds_path).map_err(|e| format!("Failed to read DDS: {}", e))?;
        if raw.len() < 128 || &raw[0..4] != b"DDS " {
            return Err("Invalid DDS header".to_string());
        }

        let height = LittleEndian::read_u32(&raw[12..16]);
        let width = LittleEndian::read_u32(&raw[16..20]);
        let mip_count = LittleEndian::read_u32(&raw[28..32]).max(1);

        details.push(AssetDetail {
            name: "width".to_string(),
            value: width.to_string(),
        });
        details.push(AssetDetail {
            name: "height".to_string(),
            value: height.to_string(),
        });
        details.push(AssetDetail {
            name: "mipCount".to_string(),
            value: mip_count.to_string(),
        });

        Ok(())
    }

    pub fn collect_tif_details(
        tif_path: &Path,
        details: &mut Vec<AssetDetail>,
    ) -> Result<(), String> {
        if let Ok(instructions) = crate::formats::CryTifIO::read_special_instructions(tif_path)
            && !instructions.is_empty()
        {
            for token in instructions.split(' ') {
                if let Some(val) = token.strip_prefix("/cryasset=") {
                    for pair in val.split(';') {
                        let kv: Vec<&str> = pair.split(',').collect();
                        if kv.len() == 2 {
                            details.push(AssetDetail {
                                name: kv[0].to_string(),
                                value: kv[1].to_string(),
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
