use super::geom_cache_file::BlockCompressionFormat;
use flate2::Compression;
use flate2::write::ZlibEncoder;
use std::io::Write;

pub trait IGeomCacheBlockCompressor: Send + Sync {
    fn compress(&self, input: &[u8]) -> Result<Vec<u8>, String>;
}

pub struct GeomCacheStoreBlockCompressor;
impl IGeomCacheBlockCompressor for GeomCacheStoreBlockCompressor {
    fn compress(&self, input: &[u8]) -> Result<Vec<u8>, String> {
        Ok(input.to_vec())
    }
}

pub struct GeomCacheDeflateBlockCompressor;
impl IGeomCacheBlockCompressor for GeomCacheDeflateBlockCompressor {
    fn compress(&self, input: &[u8]) -> Result<Vec<u8>, String> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
        encoder
            .write_all(input)
            .map_err(|e| format!("Deflate write error: {}", e))?;
        encoder
            .finish()
            .map_err(|e| format!("Deflate finish error: {}", e))
    }
}

pub struct GeomCacheLZ4HCBlockCompressor;
impl IGeomCacheBlockCompressor for GeomCacheLZ4HCBlockCompressor {
    fn compress(&self, input: &[u8]) -> Result<Vec<u8>, String> {
        Ok(lz4_flex::compress(input))
    }
}

pub fn create_block_compressor(
    format: BlockCompressionFormat,
) -> Box<dyn IGeomCacheBlockCompressor> {
    match format {
        BlockCompressionFormat::None => Box::new(GeomCacheStoreBlockCompressor),
        BlockCompressionFormat::Deflate => Box::new(GeomCacheDeflateBlockCompressor),
        BlockCompressionFormat::Lz4Hc => Box::new(GeomCacheLZ4HCBlockCompressor),
    }
}
