//! The recording as a WAV file for the STT-Suite: mono, 16 bit, at the rate the microphone delivers. The suite
//! converts it to what its engine needs (16 kHz), so Mundwerk does not resample.

use std::io::Cursor;

use crate::Error;

/// `samples`: mono, -1.0 to 1.0.
///
/// # Errors
/// The WAV writer fails (it writes to memory, so only on an impossible rate).
pub fn encode(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>, Error> {
    let spec =
        hound::WavSpec { channels: 1, sample_rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut bytes = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut bytes, spec).map_err(|error| Error::Audio(error.to_string()))?;
    for &sample in samples {
        // Clamped first, so the cast cannot overflow.
        #[allow(clippy::cast_possible_truncation)]
        let value = (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
        writer.write_sample(value).map_err(|error| Error::Audio(error.to_string()))?;
    }
    writer.finalize().map_err(|error| Error::Audio(error.to_string()))?;
    Ok(bytes.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wav_file_with_the_samples() {
        let bytes = encode(&[0.0, 0.5, -1.0, 2.0], 48_000).unwrap();
        let mut reader = hound::WavReader::new(Cursor::new(bytes)).unwrap();
        assert_eq!(reader.spec().sample_rate, 48_000);
        assert_eq!(reader.spec().channels, 1);
        let samples: Vec<i16> = reader.samples::<i16>().map(Result::unwrap).collect();
        assert_eq!(samples, [0, 16_383, -32_767, 32_767]);
    }
}
