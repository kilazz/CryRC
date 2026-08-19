// Copyright 2006-2026 Crytek GmbH / Crytek Group. All rights reserved.

use crate::xml_converter::XmlNode;
use crate::xml_filter::{FilterType, XmlFilter};
use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use std::collections::HashMap;

pub const CRYXML_SIGNATURE: &[u8; 8] = b"CryXmlB\0";

pub struct XMLBinaryWriter<'a> {
    filter: Option<&'a XmlFilter>,
    need_swap_endian: bool,
}

impl<'a> XMLBinaryWriter<'a> {
    pub fn new(filter: Option<&'a XmlFilter>, need_swap_endian: bool) -> Self {
        Self {
            filter,
            need_swap_endian,
        }
    }

    pub fn write_node(&self, root: &XmlNode) -> Result<Vec<u8>, String> {
        let mut string_table = StringTable::default();
        let mut nodes_raw = Vec::new();
        let mut attrs_raw = Vec::new();
        let mut child_indices = Vec::new();

        self.flatten(
            root,
            u32::MAX,
            &mut string_table,
            &mut nodes_raw,
            &mut attrs_raw,
            &mut child_indices,
        );

        let string_bytes = string_table.into_bytes();

        let header_size = 8 + 4 + (4 * 2) * 4; // 44 bytes
        let node_size = 28;
        let attr_size = 8;

        let node_table_offset = header_size as u32;
        let node_table_len = (nodes_raw.len() * node_size) as u32;

        let attr_table_offset = node_table_offset + node_table_len;
        let attr_table_len = (attrs_raw.len() * attr_size) as u32;

        let child_table_offset = attr_table_offset + attr_table_len;
        let child_table_len = (child_indices.len() * 4) as u32;

        let string_table_offset = child_table_offset + child_table_len;
        let string_table_len = string_bytes.len() as u32;

        let total_file_size = string_table_offset + string_table_len;
        let mut buffer = Vec::with_capacity(total_file_size as usize);

        buffer.extend_from_slice(CRYXML_SIGNATURE);

        let write_u32 = |val: u32, buf: &mut Vec<u8>| {
            if self.need_swap_endian {
                buf.write_u32::<BigEndian>(val).unwrap();
            } else {
                buf.write_u32::<LittleEndian>(val).unwrap();
            }
        };

        let write_u16 = |val: u16, buf: &mut Vec<u8>| {
            if self.need_swap_endian {
                buf.write_u16::<BigEndian>(val).unwrap();
            } else {
                buf.write_u16::<LittleEndian>(val).unwrap();
            }
        };

        // Header
        write_u32(total_file_size, &mut buffer);
        write_u32(node_table_offset, &mut buffer);
        write_u32(nodes_raw.len() as u32, &mut buffer);
        write_u32(attr_table_offset, &mut buffer);
        write_u32(attrs_raw.len() as u32, &mut buffer);
        write_u32(child_table_offset, &mut buffer);
        write_u32(child_indices.len() as u32, &mut buffer);
        write_u32(string_table_offset, &mut buffer);
        write_u32(string_table_len, &mut buffer);

        // Node Table
        for n in &nodes_raw {
            write_u32(n.tag_offset, &mut buffer);
            write_u32(n.content_offset, &mut buffer);
            write_u16(n.attr_count, &mut buffer);
            write_u16(n.child_count, &mut buffer);
            write_u32(n.parent_idx, &mut buffer);
            write_u32(n.first_attr_idx, &mut buffer);
            write_u32(n.first_child_idx, &mut buffer);
        }

        // Attribute Table
        for a in &attrs_raw {
            write_u32(a.key_offset, &mut buffer);
            write_u32(a.val_offset, &mut buffer);
        }

        // Child Index Table
        for c in &child_indices {
            write_u32(*c, &mut buffer);
        }

        // String Table
        buffer.extend_from_slice(&string_bytes);

        Ok(buffer)
    }

    fn flatten(
        &self,
        node: &XmlNode,
        parent_idx: u32,
        strings: &mut StringTable,
        nodes: &mut Vec<RawNode>,
        attrs: &mut Vec<RawAttr>,
        child_indices: &mut Vec<u32>,
    ) -> u32 {
        let current_node_idx = nodes.len() as u32;

        let tag_offset = strings.insert(&node.tag);
        let content_offset = strings.insert(&node.content);

        let first_attr_idx = attrs.len() as u32;
        let mut attr_count = 0;

        for (k, v) in &node.attributes {
            if let Some(f) = self.filter
                && !f.is_accepted(FilterType::AttributeName, k)
            {
                continue;
            }
            let key_offset = strings.insert(k);
            let val_offset = strings.insert(v);
            attrs.push(RawAttr {
                key_offset,
                val_offset,
            });
            attr_count += 1;
        }

        nodes.push(RawNode {
            tag_offset,
            content_offset,
            attr_count,
            child_count: 0,
            parent_idx,
            first_attr_idx,
            first_child_idx: 0,
        });

        let mut filtered_children = Vec::new();
        for child in &node.children {
            if let Some(f) = self.filter
                && !f.is_accepted(FilterType::ElementName, &child.tag)
            {
                continue;
            }
            filtered_children.push(child);
        }

        let first_child_idx = child_indices.len() as u32;
        let child_count = filtered_children.len() as u16;

        let child_start = child_indices.len();
        for _ in 0..filtered_children.len() {
            child_indices.push(0);
        }

        for (i, child) in filtered_children.into_iter().enumerate() {
            let idx = self.flatten(
                child,
                current_node_idx,
                strings,
                nodes,
                attrs,
                child_indices,
            );
            child_indices[child_start + i] = idx;
        }

        nodes[current_node_idx as usize].child_count = child_count;
        nodes[current_node_idx as usize].first_child_idx = first_child_idx;

        current_node_idx
    }
}

#[derive(Default)]
struct StringTable {
    map: HashMap<String, u32>,
    data: Vec<u8>,
}

impl StringTable {
    fn insert(&mut self, s: &str) -> u32 {
        if let Some(&offset) = self.map.get(s) {
            return offset;
        }
        let offset = self.data.len() as u32;
        self.data.extend_from_slice(s.as_bytes());
        self.data.push(0);
        self.map.insert(s.to_string(), offset);
        offset
    }

    fn into_bytes(self) -> Vec<u8> {
        self.data
    }
}

struct RawNode {
    tag_offset: u32,
    content_offset: u32,
    attr_count: u16,
    child_count: u16,
    parent_idx: u32,
    first_attr_idx: u32,
    first_child_idx: u32,
}

struct RawAttr {
    key_offset: u32,
    val_offset: u32,
}
