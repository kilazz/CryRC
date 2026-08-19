use cry_core::CryGuid;

#[derive(Debug, Clone, Default)]
pub struct AssetDetail {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Default)]
pub struct AssetDependency {
    pub path: String,
    pub count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SAssetMetadata {
    pub guid: CryGuid,
    pub asset_type: String,
    pub source: String,
    pub files: Vec<String>,
    pub details: Vec<AssetDetail>,
    pub dependencies: Vec<AssetDependency>,
}
