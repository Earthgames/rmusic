pub fn planar_to_interleaved(input: &[Vec<f32>], output: &mut [f32], channels: usize) {
    for (i, frame) in output.chunks_exact_mut(channels).enumerate() {
        for (channel, sample) in frame.iter_mut().enumerate() {
            *sample = input[channel][i];
        }
    }
}

pub fn interleaved_to_planar(input: &[f32], output: &mut [Vec<f32>], channels: usize) {
    for (i, frame) in input.chunks_exact(channels).enumerate() {
        for (channel, sample) in frame.iter().enumerate() {
            output[channel][i] = *sample;
        }
    }
}
