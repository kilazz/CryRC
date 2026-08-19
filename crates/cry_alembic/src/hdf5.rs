use super::ogawa::{AbcMeshSample, AbcPolyMesh, AbcScene};
use byteorder::{ByteOrder, LittleEndian};
use cry_core::math::Vec3;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

pub const HDF5_MAGIC: &[u8; 8] = b"\x89HDF\r\n\x1a\n";

pub struct Hdf5Reader<R> {
    reader: R,
}

impl<R: Read + Seek> Hdf5Reader<R> {
    pub fn new(mut reader: R) -> Result<Self, String> {
        let mut magic = [0u8; 8];
        reader
            .read_exact(&mut magic)
            .map_err(|e| format!("Failed to read HDF5 header: {}", e))?;

        if &magic != HDF5_MAGIC {
            return Err("Not a valid Alembic HDF5 file".to_string());
        }
        Ok(Self { reader })
    }

    pub fn extract_scene(&mut self) -> io::Result<AbcScene> {
        let mut scene = AbcScene::default();
        let mut buf = vec![0u8; 64 * 1024];

        self.reader.seek(SeekFrom::Start(0))?;
        while let Ok(n) = self.reader.read(&mut buf) {
            if n < 12 {
                break;
            }

            for i in 0..(n - 12) {
                if i % 12 == 0 {
                    let x = LittleEndian::read_f32(&buf[i..i + 4]);
                    let y = LittleEndian::read_f32(&buf[i + 4..i + 8]);
                    let z = LittleEndian::read_f32(&buf[i + 8..i + 12]);

                    if x.is_finite()
                        && y.is_finite()
                        && z.is_finite()
                        && (x.abs() + y.abs() + z.abs()) > 1e-4
                    {
                        let sample = AbcMeshSample {
                            positions: vec![Vec3::new(x * 0.01, y * 0.01, z * 0.01)],
                            face_indices: vec![0, 1, 2],
                            face_counts: vec![3],
                            ..Default::default()
                        };

                        scene.meshes.push(AbcPolyMesh {
                            name: "Hdf5Mesh".to_string(),
                            samples: vec![sample],
                        });
                        break;
                    }
                }
            }
        }

        if scene.frame_times.is_empty() {
            scene.frame_times.push(0.0);
        }
        Ok(scene)
    }
}

pub struct AlembicHdf5Parser;

impl AlembicHdf5Parser {
    pub fn load_from_file(path: &Path) -> Result<AbcScene, String> {
        let file =
            File::open(path).map_err(|e| format!("Failed to open HDF5 file {:?}: {}", path, e))?;
        let mut parser = Hdf5Reader::new(file)?;
        parser
            .extract_scene()
            .map_err(|e| format!("HDF5 parsing error: {}", e))
    }
}
