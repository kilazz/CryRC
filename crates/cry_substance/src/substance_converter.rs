use super::crytif_writer::CryTiffWriter;
use super::substance_ffi::{GeneratedOutputData, ISubstanceInstanceRenderer, SubstanceTexture16};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub struct SubstanceRenderer {
    pub generated_pairs: Vec<(PathBuf, PathBuf)>,
}

impl Default for SubstanceRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl SubstanceRenderer {
    pub fn new() -> Self {
        Self {
            generated_pairs: Vec::new(),
        }
    }
}

impl ISubstanceInstanceRenderer for SubstanceRenderer {
    fn on_output_available(
        &mut self,
        data: &GeneratedOutputData,
        texture: &SubstanceTexture16,
    ) -> Result<(), String> {
        let command_line = format!(
            "/autooptimizefile=0 /preset={} /reduce=0 /cryasset=parent,{}",
            data.texture_preset, data.source
        );

        let temp_path = data.path.with_extension("$ti");
        let final_path = data.path.with_extension("tif");

        if let Some(parent) = temp_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        CryTiffWriter::save_crytif_16bit(
            &temp_path,
            texture.width,
            texture.height,
            &texture.buffer,
            &command_line,
        )
        .map_err(|e| format!("Failed to write CryTIF: {}", e))?;

        if final_path.exists() {
            let _ = fs::remove_file(&final_path);
        }
        fs::rename(&temp_path, &final_path)
            .map_err(|e| format!("Failed to move temp file to final: {}", e))?;
        self.generated_pairs.push((data.path.clone(), final_path));
        Ok(())
    }
}

pub struct SubstanceConverter {
    pub game_root_path: PathBuf,
    pub loaded_images: HashMap<String, SubstanceTexture16>,
}

impl SubstanceConverter {
    pub fn new(game_root_path: PathBuf) -> Self {
        Self {
            game_root_path,
            loaded_images: HashMap::new(),
        }
    }

    pub fn get_input_image(&mut self, image_rel_path: &str) -> SubstanceTexture16 {
        if let Some(img) = self.loaded_images.get(image_rel_path) {
            return img.clone();
        }

        let base_path = self.game_root_path.join(image_rel_path);
        let stem = base_path.with_extension("");

        let candidates = [
            stem.with_extension("tif"),
            stem.with_extension("png"),
            stem.with_extension("tga"),
            stem.with_extension("jpg"),
            stem.with_extension("bmp"),
            stem.with_extension("dds"),
        ];

        for path in &candidates {
            if path.exists()
                && let Ok(dyn_img) = image::open(path)
            {
                let rgba = dyn_img.to_rgba16();
                let (w, h) = rgba.dimensions();
                let texture = SubstanceTexture16 {
                    width: w,
                    height: h,
                    buffer: rgba.into_raw(),
                };
                self.loaded_images
                    .insert(image_rel_path.to_string(), texture.clone());
                return texture;
            }
        }

        let fallback = SubstanceTexture16 {
            width: 16,
            height: 16,
            buffer: vec![0xFFFF; 16 * 16 * 4],
        };
        self.loaded_images
            .insert(image_rel_path.to_string(), fallback.clone());
        fallback
    }
}
