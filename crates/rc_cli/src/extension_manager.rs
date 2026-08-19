use std::collections::HashMap;

#[derive(Default)]
pub struct ExtensionManager {
    converters: HashMap<String, String>,
}

impl ExtensionManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            converters: HashMap::new(),
        };
        mgr.register_converter("ImageConverter", &["tif", "tiff", "hdr", "dds"]);
        mgr.register_converter("StatCGFCompiler", &["cgf", "cga", "i_cgf"]);
        mgr.register_converter("CharacterCompiler", &["chr", "skin", "cdf"]);
        mgr.register_converter("AnimationCompiler", &["caf", "i_caf", "dba"]);
        mgr.register_converter("XMLCompiler", &["xml"]);
        mgr.register_converter("AlembicCompiler", &["abc", "cbc"]);
        mgr.register_converter("SubstanceCompiler", &["crysub"]);
        mgr.register_converter("LuaCompiler", &["lua"]);
        mgr.register_converter("ColladaCompiler", &["dae"]);
        mgr.register_converter("ChunkCompiler", &["chunk"]);
        mgr
    }

    pub fn register_converter(&mut self, name: &str, extensions: &[&str]) {
        for &ext in extensions {
            self.converters
                .insert(ext.to_ascii_lowercase(), name.to_string());
        }
    }

    pub fn find_converter(&self, ext: &str) -> Option<&str> {
        self.converters
            .get(&ext.to_ascii_lowercase())
            .map(|s| s.as_str())
    }
}
