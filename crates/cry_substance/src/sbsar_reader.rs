use super::substance_ffi::SubstanceTexture16;
use byteorder::{ByteOrder, LittleEndian};
use flate2::read::DeflateDecoder;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct SbsarOutputGraph {
    pub identifier: String,
    pub label: String,
    pub format: String,
    pub default_width: u32,
    pub default_height: u32,
}

#[derive(Debug, Clone, Default)]
pub struct SbsarPackage {
    pub package_name: String,
    pub outputs: Vec<SbsarOutputGraph>,
    pub embedded_textures: HashMap<String, SubstanceTexture16>,
}

#[derive(Debug, Clone)]
pub struct SbsarZipEntry {
    pub file_name: String,
    pub data_offset: u64,
    pub comp_size: usize,
    pub uncomp_size: usize,
    pub method: u16,
}

pub struct SbsarReader;

impl SbsarReader {
    pub fn load_from_file(path: &Path) -> Result<SbsarPackage, String> {
        let file =
            File::open(path).map_err(|e| format!("Failed to open SBSAR {:?}: {}", path, e))?;
        let mut reader = BufReader::new(file);

        let mut package = SbsarPackage {
            package_name: path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            outputs: Vec::new(),
            embedded_textures: HashMap::new(),
        };

        let entries = Self::parse_zip_entries(&mut reader)?;

        for entry in entries {
            if entry.file_name.ends_with(".xml")
                || entry.file_name.contains("description")
                || entry.file_name.contains("graph")
            {
                reader
                    .seek(SeekFrom::Start(entry.data_offset))
                    .map_err(|e| e.to_string())?;
                let mut comp_data = vec![0u8; entry.comp_size];
                reader
                    .read_exact(&mut comp_data)
                    .map_err(|e| e.to_string())?;

                let uncompressed = if entry.method == 8 {
                    let mut decoder = DeflateDecoder::new(&comp_data[..]);
                    let mut out = Vec::with_capacity(entry.uncomp_size);
                    decoder.read_to_end(&mut out).map_err(|e| e.to_string())?;
                    out
                } else {
                    comp_data
                };

                let xml_text = String::from_utf8_lossy(&uncompressed);
                Self::parse_graph_xml(&xml_text, &mut package)?;
            }
        }

        if package.outputs.is_empty() {
            package.outputs = vec![
                SbsarOutputGraph {
                    identifier: "diffuse".to_string(),
                    label: "Albedo".to_string(),
                    format: "RGBA16".to_string(),
                    default_width: 1024,
                    default_height: 1024,
                },
                SbsarOutputGraph {
                    identifier: "normal".to_string(),
                    label: "NormalsWithSmoothness".to_string(),
                    format: "RGBA16".to_string(),
                    default_width: 1024,
                    default_height: 1024,
                },
                SbsarOutputGraph {
                    identifier: "specular".to_string(),
                    label: "Reflectance".to_string(),
                    format: "RGBA16".to_string(),
                    default_width: 1024,
                    default_height: 1024,
                },
            ];
        }

        Ok(package)
    }

    fn parse_graph_xml(xml: &str, package: &mut SbsarPackage) -> Result<(), String> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if tag == "output" || tag == "channel" {
                        let mut ident = String::new();
                        let mut label = String::new();
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"identifier" | b"id" => {
                                    ident = String::from_utf8_lossy(&attr.value).to_string()
                                }
                                b"label" | b"role" => {
                                    label = String::from_utf8_lossy(&attr.value).to_string()
                                }
                                _ => {}
                            }
                        }
                        if !ident.is_empty() {
                            let preset = match label.to_ascii_lowercase().as_str() {
                                "basecolor" | "diffuse" | "albedo" => "Albedo",
                                "normal" | "norm" => "NormalsWithSmoothness",
                                "roughness" | "gloss" | "specular" => "Reflectance",
                                _ => "Albedo",
                            };

                            package.outputs.push(SbsarOutputGraph {
                                identifier: ident,
                                label: preset.to_string(),
                                format: "RGBA16".to_string(),
                                default_width: 1024,
                                default_height: 1024,
                            });
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(err) => return Err(format!("SBSAR XML error: {}", err)),
                _ => {}
            }
            buf.clear();
        }
        Ok(())
    }

    fn parse_zip_entries<R: Read + Seek>(reader: &mut R) -> Result<Vec<SbsarZipEntry>, String> {
        let mut entries = Vec::new();
        let mut sig_buf = [0u8; 4];

        while reader.read_exact(&mut sig_buf).is_ok() {
            let sig = LittleEndian::read_u32(&sig_buf);
            if sig == 0x04034b50 {
                let mut header = [0u8; 26];
                reader.read_exact(&mut header).map_err(|e| e.to_string())?;

                let method = LittleEndian::read_u16(&header[4..6]);
                let comp_size = LittleEndian::read_u32(&header[14..18]) as usize;
                let uncomp_size = LittleEndian::read_u32(&header[18..22]) as usize;
                let name_len = LittleEndian::read_u16(&header[22..24]) as usize;
                let extra_len = LittleEndian::read_u16(&header[24..26]) as usize;

                let mut name_buf = vec![0u8; name_len];
                reader
                    .read_exact(&mut name_buf)
                    .map_err(|e| e.to_string())?;
                let file_name = String::from_utf8_lossy(&name_buf).to_string();

                if extra_len > 0 {
                    reader
                        .seek(SeekFrom::Current(extra_len as i64))
                        .map_err(|e| e.to_string())?;
                }

                let data_offset = reader.stream_position().map_err(|e| e.to_string())?;
                entries.push(SbsarZipEntry {
                    file_name,
                    data_offset,
                    comp_size,
                    uncomp_size,
                    method,
                });
                reader
                    .seek(SeekFrom::Current(comp_size as i64))
                    .map_err(|e| e.to_string())?;
            } else {
                break;
            }
        }
        Ok(entries)
    }
}
