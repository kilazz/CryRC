use super::asset_metadata::{AssetDependency, AssetDetail, SAssetMetadata};
use super::cryasset::CAsset;
use cry_core::CryGuid;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub type FnDetailsProvider =
    Arc<dyn Fn(&Path, &mut Vec<AssetDetail>, &mut Vec<AssetDependency>) -> bool + Send + Sync>;

#[derive(Debug, Default)]
pub struct CDictionary {
    pub entries: HashMap<String, String>,
}

impl CDictionary {
    pub fn from_string(key_value_pairs: &str) -> Self {
        let mut entries = HashMap::new();
        for pair in key_value_pairs.split(';') {
            let kv: Vec<&str> = pair.split(',').collect();
            if kv.len() == 2 {
                entries.insert(kv[0].trim().to_ascii_lowercase(), kv[1].trim().to_string());
            }
        }
        Self { entries }
    }

    pub fn get_value<'a>(&'a self, key: &str, default_val: &'a str) -> &'a str {
        self.entries
            .get(&key.to_ascii_lowercase())
            .map(|s| s.as_str())
            .unwrap_or(default_val)
    }
}

pub struct AssetManager {
    providers: HashMap<String, FnDetailsProvider>,
    asset_types: CDictionary,
}

impl Default for AssetManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetManager {
    pub fn new() -> Self {
        let default_types = "dds,Texture;tif,Texture;cgf,Mesh;cga,AnimatedMesh;skin,Skin;chr,Skeleton;caf,Animation;i_caf,Animation;mtl,Material;cdf,CharacterDefinition;xml,Xml";
        Self {
            providers: HashMap::new(),
            asset_types: CDictionary::from_string(default_types),
        }
    }

    pub fn register_detail_provider<F>(&mut self, ext: &str, provider: F)
    where
        F: Fn(&Path, &mut Vec<AssetDetail>, &mut Vec<AssetDependency>) -> bool
            + Send
            + Sync
            + 'static,
    {
        self.providers
            .insert(ext.to_ascii_lowercase(), Arc::new(provider));
    }

    pub fn get_metadata_filename(asset_path: &Path) -> PathBuf {
        let mut filename = asset_path.as_os_str().to_os_string();
        filename.push(".cryasset");
        PathBuf::from(filename)
    }

    pub fn get_metadata_type(&self, ext: &str) -> String {
        self.asset_types.get_value(ext, ext).to_string()
    }

    pub fn update_files(
        &self,
        metadata: &mut SAssetMetadata,
        source_filepath: &Path,
        files: &[PathBuf],
        user_values_str: &str,
    ) {
        if metadata.guid.is_null() {
            metadata.guid = CryGuid::create();
        }

        if let Some(first) = files.first() {
            let ext = first
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            metadata.asset_type = self.get_metadata_type(ext);
        }

        metadata.files.clear();
        for file in files {
            if let Some(name) = file.file_name().and_then(|s| s.to_str()) {
                metadata.files.push(name.to_string());
            }
        }

        let user_dict = CDictionary::from_string(user_values_str);
        let custom_source = user_dict.get_value("source", "");

        if !custom_source.is_empty() {
            metadata.source = custom_source.to_string();
        } else if source_filepath.as_os_str() != files[0].as_os_str()
            && let Some(src_name) = source_filepath.file_name().and_then(|s| s.to_str())
        {
            metadata.source = src_name.to_string();
        }

        let parent_asset_path = Self::get_metadata_filename(source_filepath);
        if parent_asset_path.exists() {
            let mut parent_asset = CAsset::new();
            if parent_asset.read_from_file(&parent_asset_path).is_ok()
                && !parent_asset.metadata.source.is_empty()
            {
                metadata.source = parent_asset.metadata.source;
            }
        }
    }

    pub fn save_cryasset(
        &self,
        source_filepath: &Path,
        files: &[PathBuf],
        output_folder: Option<&Path>,
        strip_metadata: bool,
        user_cryasset_options: &str,
    ) -> Result<PathBuf, String> {
        if strip_metadata || files.is_empty() {
            return Ok(PathBuf::new());
        }

        let primary_output = &files[0];
        let meta_name = Self::get_metadata_filename(primary_output);

        let final_meta_path = if let Some(out_dir) = output_folder {
            let file_name = meta_name.file_name().unwrap_or_default();
            out_dir.join(file_name)
        } else {
            meta_name
        };

        let mut asset = CAsset::new();
        if final_meta_path.exists() {
            let _ = asset.read_from_file(&final_meta_path);
        }

        self.update_files(
            &mut asset.metadata,
            source_filepath,
            files,
            user_cryasset_options,
        );
        self.collect_metadata_details(primary_output, &mut asset.metadata);

        if source_filepath.exists() && source_filepath != primary_output {
            self.collect_metadata_details(source_filepath, &mut asset.metadata);
        }

        asset.save_to_file(&final_meta_path)?;
        Ok(final_meta_path)
    }

    fn collect_metadata_details(&self, path: &Path, metadata: &mut SAssetMetadata) {
        if let Some(ext) = path.extension().and_then(|s| s.to_str())
            && let Some(provider) = self.providers.get(&ext.to_ascii_lowercase())
        {
            provider(path, &mut metadata.details, &mut metadata.dependencies);
        }
    }
}
