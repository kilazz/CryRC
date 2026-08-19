// Copyright 2006-2026 Crytek GmbH / Crytek Group. All rights reserved.

use crate::xml_converter::XmlNode;
use byteorder::{ByteOrder, LittleEndian};
use std::io::Read;
use thiserror::Error;

/// CryEngine binary XML file format signature ('CryXmlB\0')
pub const CRYXML_SIGNATURE: &[u8; 8] = b"CryXmlB\0";

#[derive(Error, Debug)]
pub enum BinaryReaderError {
    #[error("Not a CryXmlB file (signature mismatch)")]
    InvalidSignature,
    #[error("Corrupted binary data: {0}")]
    CorruptedData(&'static str),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct XMLBinaryReader;

impl XMLBinaryReader {
    /// Inspects whether the provided byte slice begins with the valid `CryXmlB\0` signature
    pub fn is_binary_xml(data: &[u8]) -> bool {
        data.len() >= 8 && &data[0..8] == CRYXML_SIGNATURE
    }

    /// Parses and reconstructs the XML DOM tree from raw `CryXmlB` binary bytes
    pub fn load_from_bytes(data: &[u8]) -> Result<XmlNode, BinaryReaderError> {
        if !Self::is_binary_xml(data) {
            return Err(BinaryReaderError::InvalidSignature);
        }

        let mut cursor = &data[8..];
        let read_u32 = |c: &mut &[u8]| -> Result<u32, BinaryReaderError> {
            let mut buf = [0u8; 4];
            c.read_exact(&mut buf)?;
            Ok(LittleEndian::read_u32(&buf))
        };

        let _file_size = read_u32(&mut cursor)?;
        let node_offset = read_u32(&mut cursor)? as usize;
        let node_count = read_u32(&mut cursor)? as usize;
        let attr_offset = read_u32(&mut cursor)? as usize;
        let _attr_count = read_u32(&mut cursor)? as usize;
        let child_offset = read_u32(&mut cursor)? as usize;
        let _child_count = read_u32(&mut cursor)? as usize;
        let string_offset = read_u32(&mut cursor)? as usize;
        let string_size = read_u32(&mut cursor)? as usize;

        if string_offset + string_size > data.len() {
            return Err(BinaryReaderError::CorruptedData(
                "String table out of bounds",
            ));
        }

        let string_table = &data[string_offset..string_offset + string_size];
        let get_str = |off: usize| -> Result<String, BinaryReaderError> {
            if off >= string_table.len() {
                return Err(BinaryReaderError::CorruptedData(
                    "String offset out of bounds",
                ));
            }
            let bytes = &string_table[off..];
            let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            Ok(String::from_utf8_lossy(&bytes[..end]).to_string())
        };

        let mut nodes = Vec::with_capacity(node_count);
        let mut meta = Vec::with_capacity(node_count);

        // 1. Read node descriptors (28 bytes each)
        for i in 0..node_count {
            let start = node_offset + i * 28;
            if start + 28 > data.len() {
                return Err(BinaryReaderError::CorruptedData("Node table out of bounds"));
            }
            let slice = &data[start..start + 28];
            let tag_off = LittleEndian::read_u32(&slice[0..4]) as usize;
            let content_off = LittleEndian::read_u32(&slice[4..8]) as usize;
            let attr_cnt = LittleEndian::read_u16(&slice[8..10]);
            let child_cnt = LittleEndian::read_u16(&slice[10..12]);
            let first_attr = LittleEndian::read_u32(&slice[16..20]);
            let first_child = LittleEndian::read_u32(&slice[20..24]);

            let mut node = XmlNode::new(get_str(tag_off)?);
            node.content = get_str(content_off)?;

            nodes.push(node);
            meta.push((first_attr, attr_cnt, first_child, child_cnt));
        }

        // 2. Resolve attributes and child relationships
        for i in 0..node_count {
            let (first_attr, attr_cnt, first_child, child_cnt) = meta[i];

            for a in 0..attr_cnt {
                let start = attr_offset + (first_attr as usize + a as usize) * 8;
                if start + 8 > data.len() {
                    return Err(BinaryReaderError::CorruptedData(
                        "Attribute table out of bounds",
                    ));
                }
                let slice = &data[start..start + 8];
                let key_off = LittleEndian::read_u32(&slice[0..4]) as usize;
                let val_off = LittleEndian::read_u32(&slice[4..8]) as usize;
                nodes[i]
                    .attributes
                    .push((get_str(key_off)?, get_str(val_off)?));
            }

            for c in 0..child_cnt {
                let start = child_offset + (first_child as usize + c as usize) * 4;
                if start + 4 > data.len() {
                    return Err(BinaryReaderError::CorruptedData(
                        "Child index table out of bounds",
                    ));
                }
                let child_idx = LittleEndian::read_u32(&data[start..start + 4]) as usize;
                if child_idx >= nodes.len() {
                    return Err(BinaryReaderError::CorruptedData("Child node index invalid"));
                }
                let child = nodes[child_idx].clone();
                nodes[i].children.push(child);
            }
        }

        Ok(nodes.into_iter().next().unwrap_or_else(|| XmlNode::new("")))
    }
}
