use super::asset_metadata::{AssetDependency, AssetDetail};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::fs;
use std::path::{Path, PathBuf};

pub struct AssetCollectors;

impl AssetCollectors {
    pub fn collect_cgf_details(
        cgf_path: &Path,
        details: &mut Vec<AssetDetail>,
        dependencies: &mut Vec<AssetDependency>,
    ) {
        details.push(AssetDetail {
            name: "materialCount".to_string(),
            value: "1".to_string(),
        });
        details.push(AssetDetail {
            name: "triangleCount".to_string(),
            value: "0".to_string(),
        });
        details.push(AssetDetail {
            name: "vertexCount".to_string(),
            value: "0".to_string(),
        });

        let mtl_path = cgf_path.with_extension("mtl");
        dependencies.push(AssetDependency {
            path: mtl_path.to_string_lossy().replace('\\', "/"),
            count: 1,
        });
    }

    pub fn collect_mtl_details(
        mtl_path: &Path,
        details: &mut Vec<AssetDetail>,
        dependencies: &mut Vec<AssetDependency>,
    ) -> Result<(), String> {
        let content =
            fs::read_to_string(mtl_path).map_err(|e| format!("Failed to read MTL: {}", e))?;
        let mut reader = Reader::from_str(&content);
        let mut buf = Vec::new();

        let mut sub_material_count = 0;
        let mut texture_count = 0;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if tag.eq_ignore_ascii_case("Material") {
                        sub_material_count += 1;
                    } else if tag.eq_ignore_ascii_case("Texture") {
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            if key.eq_ignore_ascii_case("File") {
                                let val = String::from_utf8_lossy(&attr.value).to_string();
                                let dds_path = PathBuf::from(val).with_extension("dds");
                                let dds_str = dds_path.to_string_lossy().replace('\\', "/");

                                if let Some(dep) = dependencies
                                    .iter_mut()
                                    .find(|d| d.path.eq_ignore_ascii_case(&dds_str))
                                {
                                    dep.count += 1;
                                } else {
                                    dependencies.push(AssetDependency {
                                        path: dds_str,
                                        count: 1,
                                    });
                                    texture_count += 1;
                                }
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(format!("Error parsing MTL XML: {}", e)),
                _ => {}
            }
            buf.clear();
        }

        details.push(AssetDetail {
            name: "subMaterialCount".to_string(),
            value: sub_material_count.to_string(),
        });
        details.push(AssetDetail {
            name: "textureCount".to_string(),
            value: texture_count.to_string(),
        });
        Ok(())
    }

    pub fn collect_xml_details(
        xml_path: &Path,
        details: &mut Vec<AssetDetail>,
    ) -> Result<(), String> {
        let content =
            fs::read_to_string(xml_path).map_err(|e| format!("Failed to read XML: {}", e))?;
        let mut reader = Reader::from_str(&content);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let root_tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    details.push(AssetDetail {
                        name: "rootTag".to_string(),
                        value: root_tag,
                    });
                    break;
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(format!("Error parsing XML: {}", e)),
                _ => {}
            }
            buf.clear();
        }
        Ok(())
    }

    pub fn collect_cdf_details(
        cdf_path: &Path,
        dependencies: &mut Vec<AssetDependency>,
    ) -> Result<(), String> {
        let content =
            fs::read_to_string(cdf_path).map_err(|e| format!("Failed to read CDF: {}", e))?;
        let mut reader = Reader::from_str(&content);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let val = String::from_utf8_lossy(&attr.value).to_string();
                        if matches!(
                            key.as_str(),
                            "File"
                                | "Material"
                                | "Binding"
                                | "simBinding"
                                | "MaterialLOD0"
                                | "MaterialLOD1"
                        ) && !val.is_empty()
                        {
                            dependencies.push(AssetDependency {
                                path: val.replace('\\', "/"),
                                count: 0,
                            });
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(format!("Error parsing CDF: {}", e)),
                _ => {}
            }
            buf.clear();
        }
        Ok(())
    }
}
