use md5::Context;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;

#[derive(Clone, Default)]
pub struct Digest {
    context: Context,
}

impl Digest {
    pub fn new() -> Self {
        Self {
            context: Context::new(),
        }
    }

    pub fn update_from_data(&mut self, data: &[u8]) {
        self.context.consume(data);
    }

    pub fn update_from_file(&mut self, path: &Path) -> io::Result<()> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut buffer = [0u8; 32768];
        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            self.context.consume(&buffer[..n]);
        }
        Ok(())
    }

    pub fn finalize(self) -> String {
        format!("{:x}", self.context.finalize())
    }
}
