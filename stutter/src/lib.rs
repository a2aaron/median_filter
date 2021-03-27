#[macro_use]
extern crate common;

use std::sync::Arc;

use vst::{
    api::Supported,
    buffer::AudioBuffer,
    host::Host,
    plugin::{CanDo, Category, HostCallback, Info, Plugin, PluginParameters},
    util::AtomicFloat,
};

use common::{ease_in_expo, make_strings};

const MAX_BUFFER_SIZE: usize = 32768; // 2^16

struct Stutter {
    params: Arc<RawParameters>,
    ringbuf_left: RingBuffer,
    ringbuf_right: RingBuffer,
    last_trigger_state: bool,
}

impl Plugin for Stutter {
    fn new(host: HostCallback) -> Self {
        Stutter {
            params: Arc::new(RawParameters::default(host)),
            ringbuf_left: RingBuffer::new(MAX_BUFFER_SIZE / 2),
            ringbuf_right: RingBuffer::new(MAX_BUFFER_SIZE / 2),
            last_trigger_state: false,
        }
    }

    fn init(&mut self) {}

    fn get_info(&self) -> Info {
        Info {
            name: "Stutter".to_string(),
            vendor: "a2aaron".to_string(),
            // Used by hosts to differentiate between plugins.
            // Don't worry much about this now - just fill in a random number.
            unique_id: 0x53545554, // "clip"
            version: 1,
            category: Category::Effect,
            parameters: ParameterType::COUNT as i32,
            // Two audio inputs
            inputs: 2,
            // Two channel audio!
            outputs: 2,
            // For now, fill in the rest of our fields with `Default` info.
            ..Default::default()
        }
    }

    fn can_do(&self, can_do: CanDo) -> Supported {
        match can_do {
            CanDo::Bypass => Supported::Yes,
            _ => Supported::No,
        }
    }

    // Output audio given the current state of the VST
    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
        let params = Parameters::from(self.params.as_ref());
        let wet_dry = params.wet_dry;
        let num_samples = buffer.samples();

        let (inputs, mut outputs) = buffer.split();
        let left_input = &inputs[0];
        let left_output = &mut outputs[0];

        self.ringbuf_left.set_size(params.buffer_size);
        self.ringbuf_right.set_size(params.buffer_size);

        match (self.last_trigger_state, params.trigger) {
            // Untriggered -> Triggered
            (false, true) => {
                self.ringbuf_left.set_triggered();
                self.ringbuf_right.set_triggered();
            }
            // Triggered -> Untriggered
            (true, false) => {
                self.ringbuf_left.set_untriggered();
                self.ringbuf_right.set_untriggered();
            }
            _ => (),
        }

        for i in 0..num_samples {
            let out = self.ringbuf_left.next(left_input[i]);
            left_output[i] = left_input[i] * (1.0 - wet_dry) + out * wet_dry;
        }

        let right_input = &inputs[1];
        let right_output = &mut outputs[1];

        for i in 0..num_samples {
            let out = self.ringbuf_right.next(right_input[i]);
            right_output[i] = right_input[i] * (1.0 - wet_dry) + out * wet_dry;
        }

        self.last_trigger_state = params.trigger;
    }

    // The raw parameters exposed to the host
    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
        Arc::clone(&self.params) as Arc<dyn PluginParameters>
    }
}

struct RingBuffer {
    buffer: [f32; MAX_BUFFER_SIZE],
    // The index of the "next" sample to be played.
    needle: usize,
    // The maximum index the needle may take on.
    size: usize,
    trigger: bool,
}

impl RingBuffer {
    fn new(size: usize) -> RingBuffer {
        RingBuffer {
            buffer: [0.0; MAX_BUFFER_SIZE],
            needle: 0,
            size,
            trigger: false,
        }
    }

    // Return the next sample from the ring buffer, optionally also consuming a
    // sample in the process.
    fn next(&mut self, input: f32) -> f32 {
        if self.trigger {
            // If the needle hasn't been through the entire buffer yet, write
            // the input. This allows `size` to increase and play the audio that
            // "would have" been there if size was larger initially.
            if self.needle < MAX_BUFFER_SIZE {
                self.buffer[self.needle] = input;
            }

            let sample = self.buffer[self.needle % self.size];
            self.needle += 1;
            sample
        } else {
            input
        }
    }

    fn set_size(&mut self, new_size: usize) {
        self.size = new_size;
    }

    fn set_triggered(&mut self) {
        self.needle = 0;
        self.trigger = true;
    }

    fn set_untriggered(&mut self) {
        self.trigger = false;
    }
}

struct Parameters {
    trigger: bool,
    buffer_size: usize,
    wet_dry: f32,
}

impl From<&RawParameters> for Parameters {
    fn from(params: &RawParameters) -> Self {
        Parameters {
            wet_dry: params.wet_dry.get(),
            buffer_size: ((ease_in_expo(params.buffer_size.get()) * MAX_BUFFER_SIZE as f32)
                as usize)
                .clamp(1, MAX_BUFFER_SIZE),
            trigger: params.trigger.get() > 0.5,
        }
    }
}

/// The raw parameter values that a host DAW will set and modify.
/// These are unscaled and are always in the [0.0, 1.0] range
pub struct RawParameters {
    wet_dry: AtomicFloat,
    trigger: AtomicFloat,
    buffer_size: AtomicFloat,
    /// The host callback, used for communicating with the VST host
    pub host: HostCallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterType {
    Trigger,
    BufferSize,
    WetDry,
}

macro_rules! table {
    ($macro:ident) => {
        $macro! {
        //  RawParameter identifier, ParameterType identifier
            RawParameters,          ParameterType;
        //  variant                     idx    name            field_name    default    strings
            ParameterType::WetDry,      0,     "Wet/Dry",      wet_dry,      1.0,       |x: f32| make_strings(x * 100.0, "%");
            ParameterType::Trigger,     1,     "Trigger",      trigger,      0.0,       |x: bool| if x {("ON".to_string(), "".to_string())} else {("OFF".to_string(), "".to_string())};
            ParameterType::BufferSize,  2,     "Buffer Size",  buffer_size,  0.5,       |x: usize| (format!("{}", x), "Samples".to_string());
        }
    };
}

impl ParameterType {
    pub const COUNT: usize = 3;
}

impl_all! {RawParameters, ParameterType, table}

// Export symbols for main
vst::plugin_main!(Stutter);
