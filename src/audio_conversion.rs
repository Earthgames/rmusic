use crate::BuF;

pub fn planar_to_interleaved(input: &[Vec<BuF>], output: &mut [BuF], channels: usize) {
    for (i, frame) in output.chunks_exact_mut(channels).enumerate() {
        for (channel, sample) in frame.iter_mut().enumerate() {
            *sample = input[channel][i];
        }
    }
}

pub fn interleaved_to_planar(input: &[BuF], output: &mut [Vec<BuF>], channels: usize) {
    for (i, frame) in input.chunks_exact(channels).enumerate() {
        for (channel, sample) in frame.iter().enumerate() {
            output[channel][i] = *sample;
        }
    }
}
