// Copyright 2006-2026 Crytek GmbH / Crytek Group. All rights reserved.

use crate::xml_binary_reader::XMLBinaryReader;
use crate::xml_binary_writer::XMLBinaryWriter;
use crate::xml_filter::{FilterType, XmlFilter};
use cry_core::io_util::CgfUtil;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::fs;
use std::path::Path;

/// In-memory generic XML node hierarchy
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlNode {
    pub tag: String,
    pub content: String,
    pub attributes: Vec<(String, String)>,
    pub children: Vec<XmlNode>,
}

impl XmlNode {
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag: tag.into(),
            content: String::new(),
            attributes: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn get_attr(&self, key: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    pub fn find_child(&self, tag: &str) -> Option<&XmlNode> {
        self.children
            .iter()
            .find(|c| c.tag.eq_ignore_ascii_case(tag))
    }
}

/// Compilation context passed to XMLCompiler
pub struct ConvertContext<'a> {
    pub source_path: &'a Path,
    pub output_path: &'a Path,
    pub filter: Option<&'a XmlFilter>,
    pub need_swap_endian: bool,
    pub force_recompile: bool,
}

/// CryEngine XML to Binary XML compiler
pub struct XMLCompiler<'a> {
    ctx: ConvertContext<'a>,
}

impl<'a> XMLCompiler<'a> {
    pub fn new(ctx: ConvertContext<'a>) -> Self {
        Self { ctx }
    }

    /// Compiles standard text XML to CryXmlB binary container with validation
    pub fn process(&self) -> Result<(), String> {
        if !self.ctx.source_path.exists() {
            return Err(format!(
                "Source file does not exist: {:?}",
                self.ctx.source_path
            ));
        }

        let raw_bytes =
            fs::read(self.ctx.source_path).map_err(|e| format!("Failed to read XML: {}", e))?;

        if XMLBinaryReader::is_binary_xml(&raw_bytes) {
            return Err(format!(
                "Source file is already binary XML: {:?}",
                self.ctx.source_path
            ));
        }

        let xml_text = String::from_utf8_lossy(&raw_bytes);
        let mut root = parse_xml_text(&xml_text)?;

        let filename_str = self.ctx.source_path.to_string_lossy();
        if let Some(filter) = self.ctx.filter
            && filter.matches_table_filemask(&filename_str)
        {
            root = convert_from_excel_xml_to_cryengine_table_xml(&root)?;
        }

        let writer = XMLBinaryWriter::new(self.ctx.filter, self.ctx.need_swap_endian);
        let binary_bytes = writer.write_node(&root).map_err(|e| e.to_string())?;

        // Perform roundtrip verification for Little-Endian host architectures
        if !self.ctx.need_swap_endian {
            let loaded_bin =
                XMLBinaryReader::load_from_bytes(&binary_bytes).map_err(|e| e.to_string())?;
            if let Err(err_info) = xmls_are_equal(&loaded_bin, &root, self.ctx.filter) {
                return Err(format!("Binary XML verification mismatch: {}", err_info));
            }
        }

        CgfUtil::write_temp_rename(self.ctx.output_path, &binary_bytes)
            .map_err(|e| format!("Failed to write binary XML: {}", e))?;

        Ok(())
    }
}

/// Converts Microsoft Excel 2003 XML spreadsheet tables into CryEngine internal table structures
pub fn convert_from_excel_xml_to_cryengine_table_xml(root: &XmlNode) -> Result<XmlNode, String> {
    let worksheet = root
        .find_child("Worksheet")
        .ok_or_else(|| "Element 'Worksheet' is missing in Excel XML".to_string())?;
    let table = worksheet
        .find_child("Table")
        .ok_or_else(|| "Element 'Table' is missing in 'Worksheet'".to_string())?;

    let mut out_table = XmlNode::new("Table");
    let mut row_index: i32 = -1;

    for node_row in &table.children {
        if !node_row.tag.eq_ignore_ascii_case("Row") {
            continue;
        }
        row_index += 1;

        if let Some(idx_str) = node_row.get_attr("ss:Index")
            && let Ok(idx) = idx_str.parse::<i32>()
        {
            let zero_based = idx - 1;
            if zero_based < row_index {
                return Err(format!("ss:Index has unexpected value {}", idx));
            }
            row_index = zero_based;
        }

        let mut row_cells = Vec::new();
        let mut cell_index: i32 = -1;

        for node_cell in &node_row.children {
            if !node_cell.tag.eq_ignore_ascii_case("Cell") {
                continue;
            }
            cell_index += 1;

            if let Some(idx_str) = node_cell.get_attr("ss:Index")
                && let Ok(idx) = idx_str.parse::<i32>()
            {
                let zero_based = idx - 1;
                if zero_based < cell_index {
                    return Err(format!("ss:Index has unexpected value {}", idx));
                }
                while cell_index < zero_based {
                    row_cells.push(String::new());
                    cell_index += 1;
                }
            }

            let cell_data = node_cell
                .find_child("Data")
                .map(|d| d.content.as_str())
                .unwrap_or("");
            row_cells.push(cell_data.to_string());
        }

        // Strip trailing empty cells
        while let Some(last) = row_cells.last() {
            if last.is_empty() {
                row_cells.pop();
            } else {
                break;
            }
        }

        if !row_cells.is_empty() {
            let mut out_row = XmlNode::new("Row");
            out_row.content = row_cells.join("\n");
            out_table.children.push(out_row);
        }
    }

    let mut out_root = XmlNode::new("Tables");
    out_root.children.push(out_table);
    Ok(out_root)
}

/// Recursively verifies that two XML node trees are semantically equal according to active filter rules
pub fn xmls_are_equal(
    node0: &XmlNode,
    node1: &XmlNode,
    filter: Option<&XmlFilter>,
) -> Result<(), String> {
    if node0.tag != node1.tag {
        return Err(format!("Tag mismatch: '{}' != '{}'", node0.tag, node1.tag));
    }

    if node0.content != node1.content {
        return Err(format!(
            "Content mismatch in <{}>: '{}' != '{}'",
            node0.tag, node0.content, node1.content
        ));
    }

    let attrs0: Vec<_> = node0
        .attributes
        .iter()
        .filter(|(k, _)| {
            filter
                .map(|f| f.is_accepted(FilterType::AttributeName, k))
                .unwrap_or(true)
        })
        .collect();

    let attrs1: Vec<_> = node1
        .attributes
        .iter()
        .filter(|(k, _)| {
            filter
                .map(|f| f.is_accepted(FilterType::AttributeName, k))
                .unwrap_or(true)
        })
        .collect();

    if attrs0.len() != attrs1.len() {
        return Err(format!("Attribute count mismatch in <{}>", node0.tag));
    }

    for ((k0, v0), (k1, v1)) in attrs0.iter().zip(attrs1.iter()) {
        if k0 != k1 || v0 != v1 {
            return Err(format!(
                "Attribute mismatch in <{}>: {}={} vs {}={}",
                node0.tag, k0, v0, k1, v1
            ));
        }
    }

    let children0: Vec<_> = node0
        .children
        .iter()
        .filter(|c| {
            filter
                .map(|f| f.is_accepted(FilterType::ElementName, &c.tag))
                .unwrap_or(true)
        })
        .collect();

    let children1: Vec<_> = node1
        .children
        .iter()
        .filter(|c| {
            filter
                .map(|f| f.is_accepted(FilterType::ElementName, &c.tag))
                .unwrap_or(true)
        })
        .collect();

    if children0.len() != children1.len() {
        return Err(format!("Child count mismatch in <{}>", node0.tag));
    }

    for (c0, c1) in children0.iter().zip(children1.iter()) {
        xmls_are_equal(c0, c1, filter)?;
    }

    Ok(())
}

fn parse_xml_text(text: &str) -> Result<XmlNode, String> {
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);

    let mut stack: Vec<XmlNode> = Vec::new();
    let mut root: Option<XmlNode> = None;
    let mut buf = Vec::new();

    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| e.to_string())?
        {
            Event::Start(e) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let mut node = XmlNode::new(tag);
                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let val = String::from_utf8_lossy(&attr.value).to_string();
                    node.attributes.push((key, val));
                }
                stack.push(node);
            }
            Event::End(_) => {
                if let Some(node) = stack.pop() {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    } else {
                        root = Some(node);
                    }
                }
            }
            Event::Empty(e) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let mut node = XmlNode::new(tag);
                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let val = String::from_utf8_lossy(&attr.value).to_string();
                    node.attributes.push((key, val));
                }
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    root = Some(node);
                }
            }
            Event::Text(e) => {
                let s = String::from_utf8_lossy(e.as_ref()).trim().to_string();
                if let Some(current) = stack.last_mut() {
                    current.content.push_str(&s);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(root.unwrap_or_else(|| XmlNode::new("root")))
}
