// Copyright 2006-2026 Crytek GmbH / Crytek Group. All rights reserved.

use crate::anim_saver::{ExportFlags, SaverAnim};
use crate::chunk_file::{CChunkFile, ChunkType};
use byteorder::{LittleEndian, WriteBytesExt};
use cry_core::CgfUtil;
use cry_core::math::Vec3;
use cry_mesh::{CMesh, MeshCompiler, MeshSubset, PhysGeomType};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
const CRYEXPORTNODE_LOWERCASE: &str = "cryexportnode_";

#[derive(Debug, Clone, Default)]
struct ColladaSource {
    floats: Vec<f32>,
    #[allow(dead_code)]
    stride: usize,
}

#[derive(Debug, Clone, Default)]
struct ColladaInput {
    semantic: String,
    source_id: String,
    offset: usize,
}

#[derive(Debug, Clone, Default)]
struct ColladaMeshPiece {
    inputs: Vec<ColladaInput>,
    vcount: Vec<usize>,
    indices: Vec<u32>,
    stride: usize,
}

#[derive(Debug, Clone, Default)]
struct ColladaGeometry {
    #[allow(dead_code)]
    position_source: String,
    sources: HashMap<String, ColladaSource>,
    pieces: Vec<ColladaMeshPiece>,
}

pub struct ColladaCompiler;

impl Default for ColladaCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl ColladaCompiler {
    pub fn new() -> Self {
        Self
    }

    pub fn process(&self, source_path: &Path, output_path: &Path) -> Result<Vec<PathBuf>, String> {
        let content = fs::read_to_string(source_path)
            .map_err(|e| format!("Failed to read Collada DAE {:?}: {}", source_path, e))?;

        let geometries = Self::parse_collada_geometries(&content)?;
        if geometries.is_empty() {
            return Err(format!(
                "No geometry found in Collada scene {:?}",
                source_path
            ));
        }

        let mut merged_mesh = CMesh::new();
        for geom in geometries.values() {
            let mut piece_mesh = CMesh::new();
            Self::convert_geom_to_cmesh(geom, &mut piece_mesh)?;
            let old_v = merged_mesh.vertex_count();
            if old_v == 0 {
                merged_mesh.copy_from(&piece_mesh);
            } else {
                merged_mesh.append_streams_from(&piece_mesh)?;
            }
        }

        if merged_mesh.subsets.is_empty() {
            merged_mesh.subsets.push(MeshSubset {
                mat_id: 0,
                physicalize_type: PhysGeomType::None,
                first_index: 0,
                num_indices: merged_mesh.indices.len() as u32,
                first_vertex: 0,
                num_vertices: merged_mesh.positions.len() as u32,
            });
        }

        MeshCompiler::compile(&mut merged_mesh, true)?;

        let mut chunk_file = CChunkFile::new();
        SaverAnim::save_export_flags(&mut chunk_file, &ExportFlags::default());
        Self::save_mesh_chunks(&mut chunk_file, &merged_mesh)?;
        Self::save_node_chunks(&mut chunk_file, output_path)?;

        let bytes = chunk_file.build_bytes().map_err(|e| e.to_string())?;
        CgfUtil::write_temp_rename(output_path, &bytes).map_err(|e| e.to_string())?;

        Ok(vec![output_path.to_path_buf()])
    }

    fn parse_collada_geometries(xml: &str) -> Result<HashMap<String, ColladaGeometry>, String> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut geometries: HashMap<String, ColladaGeometry> = HashMap::new();
        let mut cur_geom_id = String::new();
        let mut cur_source_id = String::new();
        let mut cur_source = ColladaSource::default();
        let mut cur_piece = ColladaMeshPiece::default();
        let mut current_tag = String::new();
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    current_tag = tag.clone();

                    match tag.as_str() {
                        "geometry" => {
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"id" {
                                    cur_geom_id = String::from_utf8_lossy(&attr.value).to_string();
                                    geometries
                                        .insert(cur_geom_id.clone(), ColladaGeometry::default());
                                }
                            }
                        }
                        "source" => {
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"id" {
                                    cur_source_id =
                                        String::from_utf8_lossy(&attr.value).to_string();
                                    cur_source = ColladaSource::default();
                                }
                            }
                        }
                        "triangles" | "polylist" => {
                            cur_piece = ColladaMeshPiece::default();
                        }
                        "input" => {
                            let mut input = ColladaInput::default();
                            for attr in e.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"semantic" => {
                                        input.semantic =
                                            String::from_utf8_lossy(&attr.value).to_string()
                                    }
                                    b"source" => {
                                        input.source_id = String::from_utf8_lossy(&attr.value)
                                            .trim_start_matches('#')
                                            .to_string()
                                    }
                                    b"offset" => {
                                        input.offset = String::from_utf8_lossy(&attr.value)
                                            .parse()
                                            .unwrap_or(0)
                                    }
                                    _ => {}
                                }
                            }
                            cur_piece.stride = cur_piece.stride.max(input.offset + 1);
                            cur_piece.inputs.push(input);
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(ref e)) => {
                    let text = String::from_utf8_lossy(e.as_ref());
                    match current_tag.as_str() {
                        "float_array" => {
                            cur_source.floats = text
                                .split_whitespace()
                                .filter_map(|s| s.parse().ok())
                                .collect();
                        }
                        "p" => {
                            cur_piece.indices = text
                                .split_whitespace()
                                .filter_map(|s| s.parse().ok())
                                .collect();
                        }
                        "vcount" => {
                            cur_piece.vcount = text
                                .split_whitespace()
                                .filter_map(|s| s.parse().ok())
                                .collect();
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    match tag.as_str() {
                        "source" => {
                            if let Some(geom) = geometries.get_mut(&cur_geom_id) {
                                geom.sources
                                    .insert(cur_source_id.clone(), cur_source.clone());
                            }
                        }
                        "triangles" | "polylist" => {
                            if let Some(geom) = geometries.get_mut(&cur_geom_id) {
                                geom.pieces.push(cur_piece.clone());
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(err) => return Err(format!("Collada XML error: {}", err)),
                _ => {}
            }
            buf.clear();
        }
        Ok(geometries)
    }

    fn convert_geom_to_cmesh(geom: &ColladaGeometry, out_mesh: &mut CMesh) -> Result<(), String> {
        let pos_src = geom
            .sources
            .values()
            .find(|s| s.floats.len() >= 3)
            .ok_or_else(|| "No vertex positions found in Collada geometry".to_string())?;

        for chunk in pos_src.floats.chunks_exact(3) {
            out_mesh
                .positions
                .push(Vec3::new(chunk[0], chunk[1], chunk[2]));
        }
        out_mesh
            .normals
            .resize(out_mesh.positions.len(), Vec3::new(0.0, 0.0, 1.0));
        out_mesh.uvs.resize(out_mesh.positions.len(), [0.0, 0.0]);

        for piece in &geom.pieces {
            let stride = piece.stride.max(1);
            let first_index = out_mesh.indices.len() as u32;

            if piece.vcount.is_empty() {
                for tuple in piece.indices.chunks_exact(stride) {
                    out_mesh.indices.push(tuple[0]);
                }
            } else {
                let mut p_cursor = 0;
                for &poly_verts in &piece.vcount {
                    if poly_verts >= 3 && p_cursor + poly_verts * stride <= piece.indices.len() {
                        let poly_slice = &piece.indices[p_cursor..p_cursor + poly_verts * stride];
                        for i in 1..poly_verts - 1 {
                            out_mesh.indices.push(poly_slice[0]);
                            out_mesh.indices.push(poly_slice[i * stride]);
                            out_mesh.indices.push(poly_slice[(i + 1) * stride]);
                        }
                    }
                    p_cursor += poly_verts * stride;
                }
            }

            let num_indices = out_mesh.indices.len() as u32 - first_index;
            out_mesh.subsets.push(MeshSubset {
                mat_id: 0,
                physicalize_type: PhysGeomType::None,
                first_index,
                num_indices,
                first_vertex: 0,
                num_vertices: out_mesh.positions.len() as u32,
            });
        }
        Ok(())
    }

    fn save_mesh_chunks(chunk_file: &mut CChunkFile, mesh: &CMesh) -> Result<(), String> {
        let mut data = Vec::new();
        data.write_u32::<LittleEndian>(mesh.positions.len() as u32)
            .unwrap();
        data.write_u32::<LittleEndian>(mesh.indices.len() as u32)
            .unwrap();
        data.write_u32::<LittleEndian>(mesh.subsets.len() as u32)
            .unwrap();

        for p in &mesh.positions {
            data.write_f32::<LittleEndian>(p.x).unwrap();
            data.write_f32::<LittleEndian>(p.y).unwrap();
            data.write_f32::<LittleEndian>(p.z).unwrap();
        }

        for &idx in &mesh.indices {
            data.write_u16::<LittleEndian>(idx as u16).unwrap();
        }

        for sub in &mesh.subsets {
            data.write_u32::<LittleEndian>(sub.first_index).unwrap();
            data.write_u32::<LittleEndian>(sub.num_indices).unwrap();
            data.write_u32::<LittleEndian>(sub.first_vertex).unwrap();
            data.write_u32::<LittleEndian>(sub.num_vertices).unwrap();
            data.write_i32::<LittleEndian>(sub.mat_id).unwrap();
        }

        chunk_file.add_chunk(ChunkType::Mesh, 0x0800, data);
        Ok(())
    }

    fn save_node_chunks(chunk_file: &mut CChunkFile, path: &Path) -> Result<(), String> {
        let mut data = Vec::new();
        let node_name = path.file_stem().unwrap_or_default().to_string_lossy();

        let mut name_buf = [0u8; 64];
        let bytes = node_name.as_bytes();
        let len = bytes.len().min(63);
        name_buf[..len].copy_from_slice(&bytes[..len]);
        data.extend_from_slice(&name_buf);

        data.write_i32::<LittleEndian>(0).unwrap();
        data.write_i32::<LittleEndian>(-1).unwrap();
        data.write_u32::<LittleEndian>(0).unwrap();
        data.write_i32::<LittleEndian>(0).unwrap();

        let tm = [
            1.0f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        for &val in &tm {
            data.write_f32::<LittleEndian>(val).unwrap();
        }

        data.write_i32::<LittleEndian>(-1).unwrap();
        data.write_i32::<LittleEndian>(-1).unwrap();
        data.write_i32::<LittleEndian>(-1).unwrap();
        data.write_u32::<LittleEndian>(0).unwrap();

        chunk_file.add_chunk(ChunkType::Node, 0x0824, data);
        Ok(())
    }
}
