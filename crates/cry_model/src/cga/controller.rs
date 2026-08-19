use super::quat_quantization::SmallTree64BitExtQuat;
use cry_core::math::Vec3;

#[derive(Debug, Clone)]
pub enum KeyTimesData {
    F32(Vec<f32>),
    UINT16(Vec<u16>),
    Byte(Vec<u8>),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PQLogS {
    pub position: Vec3,
    pub rotation_log: [f32; 3],
    pub scale: Vec3,
}

#[derive(Debug, Clone)]
pub struct ControllerPQLog {
    pub controller_id: u32,
    pub keys: Vec<PQLogS>,
    pub times: Vec<i32>,
}

impl ControllerPQLog {
    pub fn new(controller_id: u32) -> Self {
        Self {
            controller_id,
            keys: Vec::new(),
            times: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Controller {
    pub controller_id: u32,
    pub rot_keys: Vec<SmallTree64BitExtQuat>,
    pub rot_times: KeyTimesData,
    pub pos_keys: Vec<Vec3>,
    pub pos_times: KeyTimesData,
    pub scl_keys: Vec<Vec3>,
    pub scl_times: KeyTimesData,
}

impl Controller {
    pub fn new(controller_id: u32) -> Self {
        Self {
            controller_id,
            rot_keys: Vec::new(),
            rot_times: KeyTimesData::F32(Vec::new()),
            pos_keys: Vec::new(),
            pos_times: KeyTimesData::F32(Vec::new()),
            scl_keys: Vec::new(),
            scl_times: KeyTimesData::F32(Vec::new()),
        }
    }
}
