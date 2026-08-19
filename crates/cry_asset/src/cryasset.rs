use super::asset_metadata::{AssetDependency, AssetDetail, SAssetMetadata};
use cry_core::{CgfUtil, CryGuid};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use std::fs;
use std::io::Cursor;
use std::path::Path;

pub struct CAsset {
    pub metadata: SAssetMetadata,
}

impl Default for CAsset {
    fn default() -> Self {
        Self::new()
    }
}

impl CAsset {
    pub fn new() -> Self {
        Self {
            metadata: SAssetMetadata::default(),
        }
    }

    pub fn read_from_file(&mut self, path: &Path) -> Result<(), String> {
        if !path.exists() {
            return Err(format!("File does not exist: {:?}", path));
        }

        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read cryasset: {}", e))?;
        let mut reader = Reader::from_str(&content);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut in_files = false;
        let mut in_details = false;
        let mut in_dependencies = false;
        let mut current_dep_count = 0;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let tag = e.name();
                    match tag.as_ref() {
                        b"Asset" | b"Metadata" => {
                            for attr in e.attributes().flatten() {
                                let key = attr.key.as_ref();
                                let val = String::from_utf8_lossy(&attr.value).to_string();
                                match key {
                                    b"guid" => {
                                        if let Ok(guid) = CryGuid::parse(&val) {
                                            self.metadata.guid = guid;
                                        }
                                    }
                                    b"type" => self.metadata.asset_type = val,
                                    b"source" => self.metadata.source = val,
                                    _ => {}
                                }
                            }
                        }
                        b"Files" => in_files = true,
                        b"File" if in_files => {
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"path" {
                                    let path_val = String::from_utf8_lossy(&attr.value).to_string();
                                    if !path_val.is_empty() {
                                        self.metadata.files.push(path_val);
                                    }
                                }
                            }
                        }
                        b"Details" => in_details = true,
                        b"Detail" if in_details => {
                            let mut d_name = String::new();
                            let mut d_val = String::new();
                            for attr in e.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"name" => {
                                        d_name = String::from_utf8_lossy(&attr.value).to_string()
                                    }
                                    b"value" => {
                                        d_val = String::from_utf8_lossy(&attr.value).to_string()
                                    }
                                    _ => {}
                                }
                            }
                            if !d_name.is_empty() {
                                self.metadata.details.push(AssetDetail {
                                    name: d_name,
                                    value: d_val,
                                });
                            }
                        }
                        b"Dependencies" => in_dependencies = true,
                        b"Path" if in_dependencies => {
                            current_dep_count = 0;
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"count" {
                                    current_dep_count =
                                        String::from_utf8_lossy(&attr.value).parse().unwrap_or(0);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) if in_dependencies => {
                    let dep_path = String::from_utf8_lossy(e.as_ref()).trim().to_string();
                    if !dep_path.is_empty() {
                        self.metadata.dependencies.push(AssetDependency {
                            path: dep_path,
                            count: current_dep_count,
                        });
                    }
                }
                Ok(Event::End(e)) => match e.name().as_ref() {
                    b"Files" => in_files = false,
                    b"Details" => in_details = false,
                    b"Dependencies" => in_dependencies = false,
                    _ => {}
                },
                Ok(Event::Eof) => break,
                Err(e) => return Err(format!("Error parsing XML in cryasset: {}", e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(())
    }

    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let mut buffer = Vec::new();
        let mut writer = Writer::new_with_indent(Cursor::new(&mut buffer), b' ', 1);

        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))
            .map_err(|e| e.to_string())?;

        let mut root = BytesStart::new("Asset");
        let guid_str = self.metadata.guid.to_string();
        root.push_attribute(("guid", guid_str.as_str()));

        if !self.metadata.asset_type.is_empty() {
            root.push_attribute(("type", self.metadata.asset_type.as_str()));
        }
        if !self.metadata.source.is_empty() {
            root.push_attribute(("source", self.metadata.source.as_str()));
        }

        writer
            .write_event(Event::Start(root))
            .map_err(|e| e.to_string())?;

        // <Files>
        writer
            .write_event(Event::Start(BytesStart::new("Files")))
            .map_err(|e| e.to_string())?;
        for file in &self.metadata.files {
            let mut file_tag = BytesStart::new("File");
            file_tag.push_attribute(("path", file.as_str()));
            writer
                .write_event(Event::Empty(file_tag))
                .map_err(|e| e.to_string())?;
        }
        writer
            .write_event(Event::End(BytesEnd::new("Files")))
            .map_err(|e| e.to_string())?;

        // <Details>
        if !self.metadata.details.is_empty() {
            writer
                .write_event(Event::Start(BytesStart::new("Details")))
                .map_err(|e| e.to_string())?;
            for detail in &self.metadata.details {
                let mut d_tag = BytesStart::new("Detail");
                d_tag.push_attribute(("name", detail.name.as_str()));
                d_tag.push_attribute(("value", detail.value.as_str()));
                writer
                    .write_event(Event::Empty(d_tag))
                    .map_err(|e| e.to_string())?;
            }
            writer
                .write_event(Event::End(BytesEnd::new("Details")))
                .map_err(|e| e.to_string())?;
        }

        // <Dependencies>
        if !self.metadata.dependencies.is_empty() {
            writer
                .write_event(Event::Start(BytesStart::new("Dependencies")))
                .map_err(|e| e.to_string())?;
            for dep in &self.metadata.dependencies {
                let mut p_tag = BytesStart::new("Path");
                let count_str = dep.count.to_string();
                if dep.count > 0 {
                    p_tag.push_attribute(("count", count_str.as_str()));
                }
                writer
                    .write_event(Event::Start(p_tag))
                    .map_err(|e| e.to_string())?;
                writer
                    .write_event(Event::Text(quick_xml::events::BytesText::new(&dep.path)))
                    .map_err(|e| e.to_string())?;
                writer
                    .write_event(Event::End(BytesEnd::new("Path")))
                    .map_err(|e| e.to_string())?;
            }
            writer
                .write_event(Event::End(BytesEnd::new("Dependencies")))
                .map_err(|e| e.to_string())?;
        }

        writer
            .write_event(Event::End(BytesEnd::new("Asset")))
            .map_err(|e| e.to_string())?;

        CgfUtil::write_temp_rename(path, &buffer).map_err(|e| e.to_string())?;
        Ok(())
    }
}
