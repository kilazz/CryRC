use super::hdf5::{AlembicHdf5Parser, HDF5_MAGIC};
use byteorder::{ByteOrder, LittleEndian};
use cry_core::math::Vec3;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

pub const OGAWA_MAGIC: &[u8; 5] = b"Ogawa";

#[derive(Debug, Clone)]
pub enum OgawaData {
    Group(Vec<OgawaNode>),
    Leaf(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct OgawaNode {
    pub offset: u64,
    pub is_group: bool,
    pub data: OgawaData,
}

#[derive(Debug, Clone, Default)]
pub struct AbcMeshSample {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<[f32; 2]>,
    pub face_counts: Vec<i32>,
    pub face_indices: Vec<i32>,
}

#[derive(Debug, Clone, Default)]
pub struct AbcPolyMesh {
    pub name: String,
    pub samples: Vec<AbcMeshSample>,
}

#[derive(Debug, Clone, Default)]
pub struct AbcXformSample {
    pub transform_matrix: [f32; 16],
    pub is_identity: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AbcXform {
    pub name: String,
    pub samples: Vec<AbcXformSample>,
}

#[derive(Debug, Clone, Default)]
pub struct AbcScene {
    pub meshes: Vec<AbcPolyMesh>,
    pub xforms: Vec<AbcXform>,
    pub frame_times: Vec<f64>,
}

pub struct OgawaReader<R> {
    reader: R,
    pub root_offset: u64,
}

impl<R: Read + Seek> OgawaReader<R> {
    pub fn new(mut reader: R) -> Result<Self, String> {
        let mut magic = [0u8; 5];
        reader
            .read_exact(&mut magic)
            .map_err(|e| format!("Failed to read Ogawa header: {}", e))?;

        if &magic != OGAWA_MAGIC {
            return Err("Not a valid Alembic Ogawa file".to_string());
        }

        reader.seek(SeekFrom::Start(8)).map_err(|e| e.to_string())?;
        let mut root_buf = [0u8; 8];
        reader
            .read_exact(&mut root_buf)
            .map_err(|e| e.to_string())?;
        let root_offset = LittleEndian::read_u64(&root_buf);

        Ok(Self {
            reader,
            root_offset,
        })
    }

    pub fn read_node(&mut self, offset: u64, is_group: bool) -> io::Result<OgawaNode> {
        self.reader.seek(SeekFrom::Start(offset))?;

        if is_group {
            let mut count_buf = [0u8; 8];
            self.reader.read_exact(&mut count_buf)?;
            let child_count = LittleEndian::read_u64(&count_buf) as usize;

            let mut child_offsets = Vec::with_capacity(child_count);
            for _ in 0..child_count {
                let mut off_buf = [0u8; 8];
                self.reader.read_exact(&mut off_buf)?;
                let raw_val = LittleEndian::read_u64(&off_buf);
                let child_is_group = (raw_val & (1u64 << 63)) != 0;
                let child_off = raw_val & !(1u64 << 63);
                child_offsets.push((child_off, child_is_group));
            }

            let mut children = Vec::with_capacity(child_count);
            for (c_off, c_is_grp) in child_offsets {
                children.push(self.read_node(c_off, c_is_grp)?);
            }

            Ok(OgawaNode {
                offset,
                is_group: true,
                data: OgawaData::Group(children),
            })
        } else {
            let mut size_buf = [0u8; 8];
            self.reader.read_exact(&mut size_buf)?;
            let data_size = LittleEndian::read_u64(&size_buf) as usize;

            let mut payload = vec![0u8; data_size];
            self.reader.read_exact(&mut payload)?;

            Ok(OgawaNode {
                offset,
                is_group: false,
                data: OgawaData::Leaf(payload),
            })
        }
    }
}

pub struct AlembicOgawaParser;

impl AlembicOgawaParser {
    pub fn load_from_file(path: &Path) -> Result<AbcScene, String> {
        let mut file =
            File::open(path).map_err(|e| format!("Failed to open ABC file {:?}: {}", path, e))?;
        let mut magic = [0u8; 8];
        file.read_exact(&mut magic)
            .map_err(|e| format!("Failed to read magic: {}", e))?;

        if &magic[0..5] == OGAWA_MAGIC {
            file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
            let mut ogawa = OgawaReader::new(file)?;
            let root_node = ogawa
                .read_node(ogawa.root_offset, true)
                .map_err(|e| format!("Failed to parse Ogawa root: {}", e))?;

            let mut scene = AbcScene::default();
            Self::extract_scene_elements(&root_node, &mut scene);

            if scene.frame_times.is_empty() {
                scene.frame_times.push(0.0);
            }
            Ok(scene)
        } else if &magic == HDF5_MAGIC {
            AlembicHdf5Parser::load_from_file(path)
        } else {
            Err("Unsupported Alembic archive format (neither Ogawa nor HDF5)".to_string())
        }
    }

    fn extract_scene_elements(node: &OgawaNode, scene: &mut AbcScene) {
        if let OgawaData::Group(ref children) = node.data {
            for child in children {
                if let OgawaData::Group(ref sub_children) = child.data {
                    Self::inspect_object_group(sub_children, scene);
                }
                Self::extract_scene_elements(child, scene);
            }
        }
    }

    fn inspect_object_group(nodes: &[OgawaNode], scene: &mut AbcScene) {
        for node in nodes {
            if let OgawaData::Leaf(ref bytes) = node.data
                && bytes.len() >= 12
                && bytes.len() % 12 == 0
            {
                let vertex_count = bytes.len() / 12;
                let mut positions = Vec::with_capacity(vertex_count);

                for i in 0..vertex_count {
                    let off = i * 12;
                    let x = LittleEndian::read_f32(&bytes[off..off + 4]) * 0.01;
                    let y = LittleEndian::read_f32(&bytes[off + 4..off + 8]) * 0.01;
                    let z = LittleEndian::read_f32(&bytes[off + 8..off + 12]) * 0.01;
                    positions.push(Vec3::new(x, y, z));
                }

                let mut mesh_sample = AbcMeshSample {
                    positions,
                    ..Default::default()
                };

                let num_verts = mesh_sample.positions.len();
                if num_verts >= 3 {
                    for i in 0..num_verts {
                        mesh_sample.face_indices.push(i as i32);
                    }
                    mesh_sample.face_counts.push(num_verts as i32);
                }

                scene.meshes.push(AbcPolyMesh {
                    name: "AlembicMesh".to_string(),
                    samples: vec![mesh_sample],
                });
                break;
            }
        }
    }
}
