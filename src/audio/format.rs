// NOTE: The entire application currently assumes 16-bit signed integer PCM format.
// All audio processing, capture, and encoding is done with this format.
// If we need to support other formats in the future, this will need to be parameterized.

/// Audio format configuration
#[derive(Debug, Clone, Copy)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioFormat {
    /// Bits per sample - the entire application currently uses 16-bit signed PCM
    pub const BITS_PER_SAMPLE: u16 = 16;
}

impl Default for AudioFormat {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            channels: 1,
        }
    }
}
