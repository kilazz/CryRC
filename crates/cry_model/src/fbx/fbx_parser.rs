// Copyright 2014-2026 Crytek GmbH / Crytek Group. All rights reserved.

use super::mesh_utils::{Mesh, VertexLinks};
use super::scene::{AttributeType, IScene, NodeAttribute, SceneNode, SceneTrs};
use byteorder::{LittleEndian, ReadBytesExt};
use cry_core::math::{Matrix34, Quat, Vec3};
use flate2::read::ZlibDecoder;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::fs::File;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::path::Path;

pub const FBX_BINARY_MAGIC: &[u8; 23] = b"Kaydara FBX Binary  \x00\x1a\x00";

#[derive(Debug, Clone, PartialEq)]
pub enum FbxAttribute {
    Bool(bool),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    ArrF64(Vec<f64>),
    ArrI32(Vec<i32>),
    String(String),
    Binary(Vec<u8>),
}

impl FbxAttribute {
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            FbxAttribute::I64(v) => Some(*v),
            FbxAttribute::I32(v) => Some(*v as i64),
            _ => None,
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            FbxAttribute::F64(v) => Some(*v),
            FbxAttribute::F32(v) => Some(*v as f64),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            FbxAttribute::String(s) => Some(s.as_str()),
            _ => None,
        }
    }
    pub fn as_f64_slice(&self) -> Option<&[f64]> {
        match self {
            FbxAttribute::ArrF64(v) => Some(v.as_slice()),
            _ => None,
        }
    }
    pub fn as_i32_slice(&self) -> Option<&[i32]> {
        match self {
            FbxAttribute::ArrI32(v) => Some(v.as_slice()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FbxDomNode {
    pub name: String,
    pub attributes: Vec<FbxAttribute>,
    pub children: Vec<FbxDomNode>,
}

impl FbxDomNode {
    pub fn find_child(&self, name: &str) -> Option<&FbxDomNode> {
        self.children.iter().find(|c| c.name == name)
    }
    pub fn children_by_name<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a FbxDomNode> {
        self.children.iter().filter(move |c| c.name == name)
    }
    pub fn get_attr(&self, idx: usize) -> Option<&FbxAttribute> {
        self.attributes.get(idx)
    }
}

struct BinaryFbxReader<R> {
    reader: R,
    version: u32,
}

impl<R: Read + Seek> BinaryFbxReader<R> {
    pub fn new(mut reader: R) -> Result<Self, String> {
        let mut magic = [0u8; 23];
        reader
            .read_exact(&mut magic)
            .map_err(|e| format!("Failed to read FBX header: {}", e))?;

        if &magic != FBX_BINARY_MAGIC {
            return Err("Not a valid Kaydara FBX binary file".to_string());
        }

        let version = reader
            .read_u32::<LittleEndian>()
            .map_err(|e| e.to_string())?;
        Ok(Self { reader, version })
    }

    pub fn read_node_tree(&mut self) -> Result<FbxDomNode, String> {
        let mut root = FbxDomNode {
            name: "<root>".to_string(),
            attributes: Vec::new(),
            children: Vec::new(),
        };

        while let Some(child) = self.read_node()? {
            root.children.push(child);
        }
        Ok(root)
    }

    fn read_node(&mut self) -> Result<Option<FbxDomNode>, String> {
        let (end_offset, num_attributes) = if self.version >= 7500 {
            let eo = self
                .reader
                .read_u64::<LittleEndian>()
                .map_err(|e| e.to_string())?;
            let na = self
                .reader
                .read_u64::<LittleEndian>()
                .map_err(|e| e.to_string())?;
            let _ba = self
                .reader
                .read_u64::<LittleEndian>()
                .map_err(|e| e.to_string())?;
            (eo, na)
        } else {
            let eo = self
                .reader
                .read_u32::<LittleEndian>()
                .map_err(|e| e.to_string())? as u64;
            let na = self
                .reader
                .read_u32::<LittleEndian>()
                .map_err(|e| e.to_string())? as u64;
            let _ba = self
                .reader
                .read_u32::<LittleEndian>()
                .map_err(|e| e.to_string())? as u64;
            (eo, na)
        };

        let name_len = self.reader.read_u8().map_err(|e| e.to_string())? as usize;
        if end_offset == 0 && num_attributes == 0 && name_len == 0 {
            return Ok(None);
        }

        let mut name_bytes = vec![0u8; name_len];
        self.reader
            .read_exact(&mut name_bytes)
            .map_err(|e| e.to_string())?;
        let name = String::from_utf8_lossy(&name_bytes).to_string();

        let mut attributes = Vec::with_capacity(num_attributes as usize);
        for _ in 0..num_attributes {
            attributes.push(self.read_attribute()?);
        }

        let mut children = Vec::new();
        let cur_pos = self.reader.stream_position().map_err(|e| e.to_string())?;

        if cur_pos < end_offset {
            while self.reader.stream_position().map_err(|e| e.to_string())? < end_offset {
                if let Some(child) = self.read_node()? {
                    children.push(child);
                } else {
                    break;
                }
            }
            self.reader
                .seek(SeekFrom::Start(end_offset))
                .map_err(|e| e.to_string())?;
        }

        Ok(Some(FbxDomNode {
            name,
            attributes,
            children,
        }))
    }

    fn read_attribute(&mut self) -> Result<FbxAttribute, String> {
        let type_code = self.reader.read_u8().map_err(|e| e.to_string())?;
        match type_code {
            b'C' => Ok(FbxAttribute::Bool(
                self.reader.read_u8().map_err(|e| e.to_string())? != 0,
            )),
            b'Y' => Ok(FbxAttribute::I16(
                self.reader
                    .read_i16::<LittleEndian>()
                    .map_err(|e| e.to_string())?,
            )),
            b'I' => Ok(FbxAttribute::I32(
                self.reader
                    .read_i32::<LittleEndian>()
                    .map_err(|e| e.to_string())?,
            )),
            b'L' => Ok(FbxAttribute::I64(
                self.reader
                    .read_i64::<LittleEndian>()
                    .map_err(|e| e.to_string())?,
            )),
            b'F' => Ok(FbxAttribute::F32(
                self.reader
                    .read_f32::<LittleEndian>()
                    .map_err(|e| e.to_string())?,
            )),
            b'D' => Ok(FbxAttribute::F64(
                self.reader
                    .read_f64::<LittleEndian>()
                    .map_err(|e| e.to_string())?,
            )),
            b'd' => {
                let data = self.read_array_payload::<f64, _>(|r| r.read_f64::<LittleEndian>())?;
                Ok(FbxAttribute::ArrF64(data))
            }
            b'i' => {
                let data = self.read_array_payload::<i32, _>(|r| r.read_i32::<LittleEndian>())?;
                Ok(FbxAttribute::ArrI32(data))
            }
            b'S' => {
                let len = self
                    .reader
                    .read_u32::<LittleEndian>()
                    .map_err(|e| e.to_string())? as usize;
                let mut buf = vec![0u8; len];
                self.reader
                    .read_exact(&mut buf)
                    .map_err(|e| e.to_string())?;
                Ok(FbxAttribute::String(
                    String::from_utf8_lossy(&buf).to_string(),
                ))
            }
            b'R' => {
                let len = self
                    .reader
                    .read_u32::<LittleEndian>()
                    .map_err(|e| e.to_string())? as usize;
                let mut buf = vec![0u8; len];
                self.reader
                    .read_exact(&mut buf)
                    .map_err(|e| e.to_string())?;
                Ok(FbxAttribute::Binary(buf))
            }
            other => Err(format!(
                "Unknown FBX attribute type code: '{}'",
                other as char
            )),
        }
    }

    fn read_array_payload<T, F>(&mut self, mut read_element: F) -> Result<Vec<T>, String>
    where
        F: FnMut(&mut dyn Read) -> io::Result<T>,
    {
        let array_len = self
            .reader
            .read_u32::<LittleEndian>()
            .map_err(|e| e.to_string())? as usize;
        let encoding = self
            .reader
            .read_u32::<LittleEndian>()
            .map_err(|e| e.to_string())?;
        let byte_len = self
            .reader
            .read_u32::<LittleEndian>()
            .map_err(|e| e.to_string())? as usize;

        let mut compressed_data = vec![0u8; byte_len];
        self.reader
            .read_exact(&mut compressed_data)
            .map_err(|e| e.to_string())?;
        let mut result = Vec::with_capacity(array_len);

        if encoding == 1 {
            let mut decoder = ZlibDecoder::new(Cursor::new(compressed_data));
            for _ in 0..array_len {
                result.push(read_element(&mut decoder).map_err(|e| e.to_string())?);
            }
        } else {
            let mut cursor = Cursor::new(compressed_data);
            for _ in 0..array_len {
                result.push(read_element(&mut cursor).map_err(|e| e.to_string())?);
            }
        }
        Ok(result)
    }
}

pub struct PureFbxScene {
    pub forward_up_axes: String,
    pub unit_size_in_cm: f64,
    pub nodes: Vec<SceneNode>,
    pub meshes: Vec<Mesh>,
}

impl Default for PureFbxScene {
    fn default() -> Self {
        Self {
            forward_up_axes: "-Y+Z".to_string(),
            unit_size_in_cm: 1.0,
            nodes: Vec::new(),
            meshes: Vec::new(),
        }
    }
}

impl PureFbxScene {
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open FBX: {}", e))?;
        let mut reader = BinaryFbxReader::new(file)?;
        let dom_root = reader.read_node_tree()?;

        let mut scene = Self::default();
        scene.extract_scene_from_dom(&dom_root)?;
        Ok(scene)
    }

    fn extract_scene_from_dom(&mut self, root: &FbxDomNode) -> Result<(), String> {
        let objects = root
            .find_child("Objects")
            .ok_or("FBX missing 'Objects' node")?;
        let connections = root.find_child("Connections");

        let mut model_nodes = HashMap::new();
        let mut geometry_nodes = HashMap::new();

        for obj in &objects.children {
            let id = obj.get_attr(0).and_then(|a| a.as_i64()).unwrap_or(0);
            match obj.name.as_str() {
                "Model" => {
                    model_nodes.insert(id, obj);
                }
                "Geometry" => {
                    geometry_nodes.insert(id, obj);
                }
                _ => {}
            }
        }

        let mut geom_to_model: HashMap<i64, i64> = HashMap::new();
        if let Some(conns) = connections {
            for c in conns.children_by_name("C") {
                let child_id = c.get_attr(1).and_then(|a| a.as_i64()).unwrap_or(0);
                let parent_id = c.get_attr(2).and_then(|a| a.as_i64()).unwrap_or(0);
                if geometry_nodes.contains_key(&child_id) && model_nodes.contains_key(&parent_id) {
                    geom_to_model.insert(child_id, parent_id);
                }
            }
        }

        let mut geom_id_to_mesh_idx = HashMap::new();
        for (&geom_id, &geom_node) in &geometry_nodes {
            let mesh = Self::parse_geometry_mesh(geom_node)?;
            let mesh_idx = self.meshes.len();
            self.meshes.push(mesh);
            geom_id_to_mesh_idx.insert(geom_id, mesh_idx);
        }

        for (&model_id, &model_node) in &model_nodes {
            let raw_name = model_node
                .get_attr(1)
                .and_then(|a| a.as_str())
                .unwrap_or("Model");
            let clean_name = raw_name
                .split('\x00')
                .next()
                .unwrap_or(raw_name)
                .replace("Model::", "");
            let world_transform = Self::extract_model_transform(model_node);

            let mut attributes = Vec::new();
            for (&geom_id, &m_id) in &geom_to_model {
                if m_id == model_id
                    && let Some(&mesh_idx) = geom_id_to_mesh_idx.get(&geom_id)
                {
                    attributes.push(NodeAttribute {
                        attr_type: AttributeType::Mesh,
                        index: mesh_idx,
                    });
                }
            }

            self.nodes.push(SceneNode {
                name: clean_name,
                world_transform,
                geometry_offset: Matrix34::IDENTITY,
                attributes,
                parent: -1,
                children: Vec::new(),
            });
        }

        Ok(())
    }

    fn parse_geometry_mesh(geom: &FbxDomNode) -> Result<Mesh, String> {
        let mut mesh = Mesh::default();

        if let Some(vertices_node) = geom.find_child("Vertices")
            && let Some(attr) = vertices_node.get_attr(0)
            && let Some(floats) = attr.as_f64_slice()
        {
            for chunk in floats.chunks_exact(3) {
                mesh.positions
                    .push(Vec3::new(chunk[0] as f32, chunk[1] as f32, chunk[2] as f32));
            }
        }

        mesh.links
            .resize(mesh.positions.len(), VertexLinks::default());

        if let Some(poly_node) = geom.find_child("PolygonVertexIndex")
            && let Some(attr) = poly_node.get_attr(0)
            && let Some(indices) = attr.as_i32_slice()
        {
            let mut poly_verts = Vec::new();
            for &idx in indices {
                if idx < 0 {
                    poly_verts.push((!idx) as u32);
                    if poly_verts.len() >= 3 {
                        for i in 1..(poly_verts.len() - 1) {
                            mesh.indices.push(poly_verts[0]);
                            mesh.indices.push(poly_verts[i]);
                            mesh.indices.push(poly_verts[i + 1]);
                        }
                    }
                    poly_verts.clear();
                } else {
                    poly_verts.push(idx as u32);
                }
            }
        }

        if let Some(normal_node) = geom.find_child("LayerElementNormal")
            && let Some(normals_data) = normal_node
                .find_child("Normals")
                .and_then(|n| n.get_attr(0))
            && let Some(floats) = normals_data.as_f64_slice()
        {
            for chunk in floats.chunks_exact(3) {
                mesh.normals
                    .push(Vec3::new(chunk[0] as f32, chunk[1] as f32, chunk[2] as f32));
            }
        }
        if mesh.normals.is_empty() {
            mesh.normals
                .resize(mesh.positions.len(), Vec3::new(0.0, 0.0, 1.0));
        }

        if let Some(uv_node) = geom.find_child("LayerElementUV")
            && let Some(uv_data) = uv_node.find_child("UV").and_then(|n| n.get_attr(0))
            && let Some(floats) = uv_data.as_f64_slice()
        {
            for chunk in floats.chunks_exact(2) {
                mesh.uvs.push([chunk[0] as f32, chunk[1] as f32]);
            }
        }
        if mesh.uvs.is_empty() {
            mesh.uvs.resize(mesh.positions.len(), [0.0, 0.0]);
        }

        Ok(mesh)
    }

    fn extract_model_transform(model: &FbxDomNode) -> Matrix34 {
        let mut translation = Vec3::ZERO;
        let mut rotation_deg = Vec3::ZERO;
        let mut scaling = Vec3::new(1.0, 1.0, 1.0);

        if let Some(props) = model.find_child("Properties70") {
            for p in props.children_by_name("P") {
                if let Some(prop_name) = p.get_attr(0).and_then(|a| a.as_str()) {
                    match prop_name {
                        "Lcl Translation" => {
                            let x = p.get_attr(4).and_then(|a| a.as_f64()).unwrap_or(0.0) as f32;
                            let y = p.get_attr(5).and_then(|a| a.as_f64()).unwrap_or(0.0) as f32;
                            let z = p.get_attr(6).and_then(|a| a.as_f64()).unwrap_or(0.0) as f32;
                            translation = Vec3::new(x, y, z);
                        }
                        "Lcl Rotation" => {
                            let x = p.get_attr(4).and_then(|a| a.as_f64()).unwrap_or(0.0) as f32;
                            let y = p.get_attr(5).and_then(|a| a.as_f64()).unwrap_or(0.0) as f32;
                            let z = p.get_attr(6).and_then(|a| a.as_f64()).unwrap_or(0.0) as f32;
                            rotation_deg = Vec3::new(x, y, z);
                        }
                        "Lcl Scaling" => {
                            let x = p.get_attr(4).and_then(|a| a.as_f64()).unwrap_or(1.0) as f32;
                            let y = p.get_attr(5).and_then(|a| a.as_f64()).unwrap_or(1.0) as f32;
                            let z = p.get_attr(6).and_then(|a| a.as_f64()).unwrap_or(1.0) as f32;
                            scaling = Vec3::new(x, y, z);
                        }
                        _ => {}
                    }
                }
            }
        }

        let rad_x = rotation_deg.x * PI / 180.0;
        let rad_y = rotation_deg.y * PI / 180.0;
        let rad_z = rotation_deg.z * PI / 180.0;

        let (cx, sx) = (rad_x.cos(), rad_x.sin());
        let (cy, sy) = (rad_y.cos(), rad_y.sin());
        let (cz, sz) = (rad_z.cos(), rad_z.sin());

        let mut mtx = Matrix34::IDENTITY;
        mtx.m[0] = [
            cy * cz * scaling.x,
            -cy * sz * scaling.y,
            sy * scaling.z,
            translation.x,
        ];
        mtx.m[1] = [
            (sx * sy * cz + cx * sz) * scaling.x,
            (-sx * sy * sz + cx * cz) * scaling.y,
            -sx * cy * scaling.z,
            translation.y,
        ];
        mtx.m[2] = [
            (-cx * sy * cz + sx * sz) * scaling.x,
            (cx * sy * sz + sx * cz) * scaling.y,
            cx * cy * scaling.z,
            translation.z,
        ];
        mtx
    }
}

impl IScene for PureFbxScene {
    fn get_forward_up_axes(&self) -> &str {
        &self.forward_up_axes
    }
    fn get_unit_size_in_centimeters(&self) -> f64 {
        self.unit_size_in_cm
    }
    fn get_node_count(&self) -> usize {
        self.nodes.len()
    }
    fn get_node(&self, idx: usize) -> Option<&SceneNode> {
        self.nodes.get(idx)
    }
    fn get_mesh_count(&self) -> usize {
        self.meshes.len()
    }
    fn get_mesh(&self, idx: usize) -> Option<&Mesh> {
        self.meshes.get(idx)
    }
    fn evaluate_node_local_transform(&self, node_idx: usize, _frame: i32) -> SceneTrs {
        self.nodes
            .get(node_idx)
            .map(|n| SceneTrs {
                translation: n.world_transform.get_translation(),
                rotation: Quat::IDENTITY,
                scale: Vec3::new(1.0, 1.0, 1.0),
            })
            .unwrap_or_default()
    }
}
