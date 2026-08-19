use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SubstanceTexture16 {
    pub width: u32,
    pub height: u32,
    pub buffer: Vec<u16>,
}

#[derive(Debug, Clone)]
pub struct GeneratedOutputData {
    pub source: String,
    pub path: PathBuf,
    pub texture_preset: String,
}

#[derive(Debug, Clone)]
pub struct SubstanceOutput {
    pub name: String,
    pub preset: String,
    pub output_path: PathBuf,
}

pub trait ISubstancePreset {
    fn get_file_name(&self) -> &str;
    fn get_substance_archive(&self) -> &str;
    fn get_outputs(&self) -> Vec<SubstanceOutput>;
}

pub trait ISubstanceInstanceRenderer {
    fn on_output_available(
        &mut self,
        output_data: &GeneratedOutputData,
        texture: &SubstanceTexture16,
    ) -> Result<(), String>;
}
