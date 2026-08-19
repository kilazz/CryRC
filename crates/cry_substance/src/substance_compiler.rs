use super::substance_converter::{SubstanceConverter, SubstanceRenderer};
use super::substance_ffi::{GeneratedOutputData, ISubstanceInstanceRenderer, ISubstancePreset};
use std::path::PathBuf;

pub struct SubstanceCompiler<'a> {
    pub converter: &'a mut SubstanceConverter,
    pub source_file: PathBuf,
    pub output_folder: PathBuf,
    pub force_recompile: bool,
}

impl<'a> SubstanceCompiler<'a> {
    pub fn new(
        converter: &'a mut SubstanceConverter,
        source_file: PathBuf,
        output_folder: PathBuf,
        force_recompile: bool,
    ) -> Self {
        Self {
            converter,
            source_file,
            output_folder,
            force_recompile,
        }
    }

    pub fn process<P: ISubstancePreset>(&mut self, preset: &P) -> Result<(), String> {
        if !self.source_file.exists() {
            return Err(format!(
                "Preset file does not exist: {:?}",
                self.source_file
            ));
        }

        let mut renderer = SubstanceRenderer::new();
        for out in preset.get_outputs() {
            let gen_data = GeneratedOutputData {
                source: preset.get_file_name().to_string(),
                path: self.output_folder.join(&out.name),
                texture_preset: out.preset.clone(),
            };

            let rendered_tex = self.converter.get_input_image(&out.name);
            renderer.on_output_available(&gen_data, &rendered_tex)?;
        }
        Ok(())
    }
}
