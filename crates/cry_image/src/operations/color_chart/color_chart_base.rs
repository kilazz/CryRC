pub trait ColorChart {
    fn generate_default(&mut self);
    fn generate_from_input(
        &mut self,
        width: usize,
        height: usize,
        bgra: &[u8],
        pitch: usize,
    ) -> Result<(), String>;
    fn generate_chart_image(&self) -> Option<(usize, usize, Vec<u8>)>;
}
