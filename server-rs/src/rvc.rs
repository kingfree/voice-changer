pub trait VCModel: Send + Sync {
    fn processing_sample_rate(&self) -> i32;
    fn inference(&self, input: &[i16]) -> Vec<i16>;
}

pub struct Rvc {
    sample_rate: i32,
}

impl Rvc {
    pub fn new(sample_rate: i32) -> Self {
        Self { sample_rate }
    }
}

impl VCModel for Rvc {
    fn processing_sample_rate(&self) -> i32 {
        self.sample_rate
    }

    fn inference(&self, input: &[i16]) -> Vec<i16> {
        input.to_vec()
    }
}
