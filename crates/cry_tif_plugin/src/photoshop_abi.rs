// Copyright 2004-2026 Crytek GmbH / Crytek Group. All rights reserved.
// Photoshop SDK Format Module C-ABI Definitions

use std::ffi::c_void;

// Format Module Operation Selectors
pub const FORMAT_SELECTOR_ABOUT: i16 = 0;
pub const FORMAT_SELECTOR_READ_PREPARE: i16 = 1;
pub const FORMAT_SELECTOR_READ_START: i16 = 2;
pub const FORMAT_SELECTOR_READ_CONTINUE: i16 = 3;
pub const FORMAT_SELECTOR_READ_FINISH: i16 = 4;
pub const FORMAT_SELECTOR_OPTIONS_PREPARE: i16 = 5;
pub const FORMAT_SELECTOR_OPTIONS_START: i16 = 6;
pub const FORMAT_SELECTOR_OPTIONS_CONTINUE: i16 = 7;
pub const FORMAT_SELECTOR_OPTIONS_FINISH: i16 = 8;
pub const FORMAT_SELECTOR_ESTIMATE_PREPARE: i16 = 9;
pub const FORMAT_SELECTOR_ESTIMATE_START: i16 = 10;
pub const FORMAT_SELECTOR_ESTIMATE_CONTINUE: i16 = 11;
pub const FORMAT_SELECTOR_ESTIMATE_FINISH: i16 = 12;
pub const FORMAT_SELECTOR_WRITE_PREPARE: i16 = 13;
pub const FORMAT_SELECTOR_WRITE_START: i16 = 14;
pub const FORMAT_SELECTOR_WRITE_CONTINUE: i16 = 15;
pub const FORMAT_SELECTOR_WRITE_FINISH: i16 = 16;
pub const FORMAT_SELECTOR_FILTER_FILE: i16 = 17;

// Photoshop Error Codes
pub const NO_ERR: i16 = 0;
pub const USER_CANCELED_ERR: i16 = -128;
pub const MEM_FULL_ERR: i16 = -108;
pub const FORMAT_CANNOT_READ: i16 = -30500;
pub const FORMAT_CANNOT_WRITE: i16 = -30501;

// Image Modes
pub const PLUG_IN_MODE_BITMAP: i16 = 0;
pub const PLUG_IN_MODE_GRAYSCALE: i16 = 1;
pub const PLUG_IN_MODE_INDEXED_COLOR: i16 = 2;
pub const PLUG_IN_MODE_RGB_COLOR: i16 = 3;
pub const PLUG_IN_MODE_CMYK_COLOR: i16 = 4;
pub const PLUG_IN_MODE_MULTICHANNEL: i16 = 7;

// Callback Signatures
pub type TestAbortProc = Option<unsafe extern "C" fn() -> u8>;
pub type ProgressProc = Option<unsafe extern "C" fn(done: i32, total: i32)>;
pub type HostProc = Option<unsafe extern "C" fn(selector: i16, data: *mut usize)>;
pub type AdvanceStateProc = Option<unsafe extern "C" fn() -> i16>;
pub type ColorServicesProc = Option<unsafe extern "C" fn(info: *mut c_void) -> i16>;

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct Point {
    pub v: i16,
    pub h: i16,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct Rect {
    pub top: i16,
    pub left: i16,
    pub bottom: i16,
    pub right: i16,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct VPoint {
    pub v: i32,
    pub h: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct VRect {
    pub top: i32,
    pub left: i32,
    pub bottom: i32,
    pub right: i32,
}

#[repr(C)]
pub struct FSSpec {
    pub unused: i16,
    pub padding: i16,
    pub par_id: i32,
    pub name: [u8; 256], // Pascal string format: name[0] = length, name[1..255] = chars
}

#[repr(C)]
pub struct SPPlatformFileSpecificationW {
    pub m_reference: *mut u16,
}

#[repr(C)]
pub struct PlugInMonitor {
    pub gamma: i32,
    pub red_x: i32,
    pub red_y: i32,
    pub green_x: i32,
    pub green_y: i32,
    pub blue_x: i32,
    pub blue_y: i32,
    pub white_x: i32,
    pub white_y: i32,
    pub ambient: i32,
}

#[repr(C)]
pub struct SPBasicSuite {
    pub acquire_suite: Option<
        unsafe extern "C" fn(name: *const i8, version: i32, suite: *mut *const c_void) -> i32,
    >,
    pub release_suite: Option<unsafe extern "C" fn(name: *const i8, version: i32) -> i32>,
    pub is_equal: Option<unsafe extern "C" fn(token1: *const i8, token2: *const i8) -> u8>,
    pub allocate_block: Option<unsafe extern "C" fn(size: usize, block: *mut *mut c_void) -> i32>,
    pub free_block: Option<unsafe extern "C" fn(block: *mut c_void) -> i32>,
    pub reallocate_block: Option<
        unsafe extern "C" fn(
            block: *mut c_void,
            new_size: usize,
            new_block: *mut *mut c_void,
        ) -> i32,
    >,
    pub undefined: Option<unsafe extern "C" fn() -> i32>,
}

#[repr(C)]
pub struct PSBufferSuite1 {
    pub new_proc:
        Option<unsafe extern "C" fn(requested_size: *mut u32, minimum_size: u32) -> *mut u8>,
    pub dispose_proc: Option<unsafe extern "C" fn(buffer: *mut *mut u8)>,
    pub get_size_proc: Option<unsafe extern "C" fn(buffer: *mut u8) -> u32>,
    pub get_space_proc: Option<unsafe extern "C" fn() -> u32>,
}

#[repr(C)]
pub struct PSHandleSuite2 {
    pub new_proc: Option<unsafe extern "C" fn(size: i32) -> *mut *mut u8>,
    pub dispose_proc: Option<unsafe extern "C" fn(h: *mut *mut u8)>,
    pub dispose_regular_handle_proc: Option<unsafe extern "C" fn(h: *mut *mut u8)>,
    pub set_lock_proc: Option<
        unsafe extern "C" fn(h: *mut *mut u8, lock: u8, address: *mut *mut u8, old_lock: *mut u8),
    >,
    pub get_size_proc: Option<unsafe extern "C" fn(h: *mut *mut u8) -> i32>,
    pub set_size_proc: Option<unsafe extern "C" fn(h: *mut *mut u8, new_size: i32) -> i16>,
    pub recover_space_proc: Option<unsafe extern "C" fn(size: i32)>,
}

/// Canonical `FormatRecord` binary layout as defined in Adobe Photoshop SDK (`PIFormat.h`).
#[repr(C, packed(4))]
pub struct FormatRecord {
    pub serial_number: i32,
    pub abort_proc: TestAbortProc,
    pub progress_proc: ProgressProc,
    pub max_data: i32,
    pub min_data_bytes: i32,
    pub max_data_bytes: i32,
    pub min_rsrc_bytes: i32,
    pub max_rsrc_bytes: i32,
    pub data_fork: isize,
    pub rsrc_fork: isize,
    pub file_spec: *mut FSSpec,
    pub image_mode: i16,
    pub image_size: Point,
    pub depth: i16,
    pub planes: i16,
    pub image_h_res: i32,
    pub image_v_res: i32,
    pub red_lut: [u8; 256],
    pub green_lut: [u8; 256],
    pub blue_lut: [u8; 256],
    pub data: *mut c_void,
    pub the_rect: Rect,
    pub lo_plane: i16,
    pub hi_plane: i16,
    pub col_bytes: i16,
    pub row_bytes: i32,
    pub plane_bytes: i32,
    pub plane_map: [i16; 16],
    pub can_transpose: u8,
    pub need_transpose: u8,
    pub host_sig: u32,
    pub host_proc: HostProc,
    pub host_modes: i16,
    pub revert_info: *mut *mut u8,
    pub host_new_hdl: Option<unsafe extern "C" fn(size: i32) -> *mut *mut u8>,
    pub host_dispose_hdl: Option<unsafe extern "C" fn(h: *mut *mut u8)>,
    pub image_rsrc_data: *mut *mut u8,
    pub image_rsrc_size: i32,
    pub monitor: PlugInMonitor,
    pub platform_data: *mut c_void,
    pub buffer_procs: *mut c_void,
    pub resource_procs: *mut c_void,
    pub process_event: Option<unsafe extern "C" fn(event: *mut c_void)>,
    pub display_pixels: Option<
        unsafe extern "C" fn(
            source: *const c_void,
            src_rect: *const VRect,
            dst_row: i32,
            dst_col: i32,
            ctx: *mut c_void,
        ) -> i16,
    >,
    pub handle_procs: *mut c_void,
    pub file_type: u32,
    pub color_services: ColorServicesProc,
    pub advance_state: AdvanceStateProc,
    pub property_procs: *mut c_void,
    pub image_services_procs: *mut c_void,
    pub tile_width: i16,
    pub tile_height: i16,
    pub tile_origin: Point,
    pub descriptor_parameters: *mut c_void,
    pub error_string: *mut [u8; 256],
    pub max_value: i32,
    pub s_sp_basic: *mut SPBasicSuite,
    pub plug_in_ref: *mut c_void,
    pub transparent_index: i32,
    pub icc_profile_data: *mut *mut u8,
    pub icc_profile_size: i32,
    pub can_use_icc_profiles: i32,
    pub lut_count: i32,
    pub preferred_color_modes: i32,
    pub convert_mode: i32,
    pub preferred_size: VPoint,
    pub image_index: i32,
    pub transparency_plane: i32,
    pub transparency_matting: i32,
    pub channel_port_procs: *mut c_void,
    pub document_info: *mut c_void,
    pub open_for_preview: u8,
    pub browser_rotation: i32,
    pub host_supports_32bit_coordinates: i32,
    pub plugin_using_32bit_coordinates: i32,
    pub image_size32: VPoint,
    pub the_rect32: VRect,
    pub requested_file_property: u32,
    pub file_property_value: u32,
    pub file_count: u32,
    pub xmp: *mut *mut u8,
    pub supports_skip_file: i32,
    pub extract_quick_thumbnail: u8,
    pub host_in_secondary_thread: u8,
    pub bulk_mask: u32,
    pub bulk_flags: *mut u32,
    pub preset_count: u32,
    pub preset_names: *mut *mut u8,
    pub preset_data: *mut *mut u8,
    pub settings_checksum: u32,
    pub file_spec2: *mut SPPlatformFileSpecificationW,
}
