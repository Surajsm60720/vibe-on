use std::f32::consts::PI;

/// Simple Freeverb-style Reverb Implementation
///
/// Uses 8 parallel Lowpass-Feedback Comb Filters (LBCF)
/// followed by 4 series Allpass Filters (APF).
///
/// Ref: https://ccrma.stanford.edu/~jos/pasp/Freeverb.html

const NUM_COMB: usize = 8;
const NUM_ALLPASS: usize = 4;

// Schroeder/Freeverb standard tuning (stereo spread usually handled by slightly different lengths)
// Tuning for 44.1kHz, scaled for sample rate in `new`.
const MUTED: f32 = 0.0;
const FIXED_GAIN: f32 = 0.015;
const SCALE_WET: f32 = 3.0;
const SCALE_DRY: f32 = 2.0;
const SCALE_DAMP: f32 = 0.4;
const SCALE_ROOM: f32 = 0.28;
const OFFSET_ROOM: f32 = 0.7;
const STEREO_SPREAD: usize = 23;

// Comb filter delays (samples at 44.1kHz)
const COMB_TUNING: [usize; NUM_COMB] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
// Allpass filter delays
const ALLPASS_TUNING: [usize; NUM_ALLPASS] = [225, 341, 441, 556];

struct Comb {
    buffer: Vec<f32>,
    index: usize,
    feedback: f32,
    filter_store: f32,
    damp: f32,
    damp1: f32,
    damp2: f32,
}

impl Comb {
    fn new(len: usize) -> Self {
        Self {
            buffer: vec![0.0; len],
            index: 0,
            feedback: 0.5,
            filter_store: 0.0,
            damp: 0.5,
            damp1: 0.5,
            damp2: 0.5,
        }
    }

    fn set_damp(&mut self, val: f32) {
        self.damp1 = val;
        self.damp2 = 1.0 - val;
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.index];
        self.filter_store = (output * self.damp2) + (self.filter_store * self.damp1);
        self.buffer[self.index] = input + (self.filter_store * self.feedback);
        self.index = (self.index + 1) % self.buffer.len();
        output
    }
}

struct Allpass {
    buffer: Vec<f32>,
    index: usize,
    feedback: f32,
}

impl Allpass {
    fn new(len: usize) -> Self {
        Self {
            buffer: vec![0.0; len],
            index: 0,
            feedback: 0.5,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let buffered_val = self.buffer[self.index];
        let output = -input + buffered_val;
        self.buffer[self.index] = input + (buffered_val * self.feedback);
        self.index = (self.index + 1) % self.buffer.len();
        output
    }
}

pub struct Freeverb {
    sample_rate: u32,
    gain: f32,
    room_size: f32,
    damp: f32,
    wet: f32,
    dry: f32,
    width: f32,

    // Left Channel
    comb_l: Vec<Comb>,
    allpass_l: Vec<Allpass>,

    // Right Channel
    comb_r: Vec<Comb>,
    allpass_r: Vec<Allpass>,
}

impl Freeverb {
    pub fn new(sample_rate: u32) -> Self {
        let sr_scale = sample_rate as f32 / 44100.0;

        let mut comb_l = Vec::with_capacity(NUM_COMB);
        let mut comb_r = Vec::with_capacity(NUM_COMB);
        let mut allpass_l = Vec::with_capacity(NUM_ALLPASS);
        let mut allpass_r = Vec::with_capacity(NUM_ALLPASS);

        for &len in COMB_TUNING.iter() {
            let scaled_len = (len as f32 * sr_scale) as usize;
            comb_l.push(Comb::new(scaled_len));
            comb_r.push(Comb::new(scaled_len + STEREO_SPREAD));
        }

        for &len in ALLPASS_TUNING.iter() {
            let scaled_len = (len as f32 * sr_scale) as usize;
            allpass_l.push(Allpass::new(scaled_len));
            allpass_r.push(Allpass::new(scaled_len + STEREO_SPREAD));
        }

        let mut rv = Self {
            sample_rate,
            gain: FIXED_GAIN,
            room_size: 0.5,
            damp: 0.5,
            wet: 1.0 / SCALE_WET, // Default nice wet level
            dry: 0.0,             // Usually handle dry outside, but Freeverb can mix
            width: 1.0,
            comb_l,
            comb_r,
            allpass_l,
            allpass_r,
        };

        rv.update();
        rv
    }

    pub fn set_room_size(&mut self, value: f32) {
        self.room_size = (value * SCALE_ROOM) + OFFSET_ROOM;
        self.update();
    }

    pub fn set_damp(&mut self, value: f32) {
        self.damp = value * SCALE_DAMP;
        self.update();
    }

    pub fn set_wet(&mut self, value: f32) {
        self.wet = value / SCALE_WET;
    }

    pub fn set_dry(&mut self, value: f32) {
        self.dry = value * SCALE_DRY;
    }

    pub fn set_width(&mut self, value: f32) {
        self.width = value;
    }

    fn update(&mut self) {
        for comb in self.comb_l.iter_mut().chain(self.comb_r.iter_mut()) {
            comb.feedback = self.room_size;
            comb.set_damp(self.damp);
        }
    }

    pub fn process(&mut self, input_l: f32, input_r: f32) -> (f32, f32) {
        let input = (input_l + input_r) * self.gain;

        let mut out_l = 0.0;
        let mut out_r = 0.0;

        // Parallel Comb Filters
        for i in 0..NUM_COMB {
            out_l += self.comb_l[i].process(input);
            out_r += self.comb_r[i].process(input);
        }

        // Series Allpass Filters
        for i in 0..NUM_ALLPASS {
            out_l = self.allpass_l[i].process(out_l);
            out_r = self.allpass_r[i].process(out_r);
        }

        // Output Mix
        let wet_1 = self.wet * (self.width / 2.0 + 0.5);
        let wet_2 = self.wet * ((1.0 - self.width) / 2.0);

        let final_l = (out_l * wet_1) + (out_r * wet_2) + (input_l * self.dry);
        let final_r = (out_r * wet_1) + (out_l * wet_2) + (input_r * self.dry);

        (final_l, final_r)
    }
}
