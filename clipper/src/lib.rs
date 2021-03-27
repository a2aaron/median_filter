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

struct Clipper {
    params: Arc<RawParameters>,
}

impl Plugin for Clipper {
    fn new(host: HostCallback) -> Self {
        Clipper {
            params: Arc::new(RawParameters::default(host)),
        }
    }

    fn init(&mut self) {}

    fn get_info(&self) -> Info {
        Info {
            name: "Clipper".to_string(),
            vendor: "a2aaron".to_string(),
            // Used by hosts to differentiate between plugins.
            // Don't worry much about this now - just fill in a random number.
            unique_id: 0x636c6970, // "clip"
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

        for i in 0..num_samples {
            let out = left_input[i] * params.pre_amplify;
            let out = out.clamp(-params.clip_level, params.clip_level);
            let out = out * params.post_amplify;
            left_output[i] = left_input[i] * (1.0 - wet_dry) + out * wet_dry;
        }

        let right_input = &inputs[1];
        let right_output = &mut outputs[1];

        for i in 0..num_samples {
            let out = right_input[i] * params.pre_amplify;
            let out = out.clamp(-params.clip_level, params.clip_level);
            let out = out * params.post_amplify;
            right_output[i] = right_input[i] * (1.0 - wet_dry) + out * wet_dry;
        }
    }

    // The raw parameters exposed to the host
    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
        Arc::clone(&self.params) as Arc<dyn PluginParameters>
    }
}

struct Parameters {
    clip_level: f32,
    pre_amplify: f32,
    post_amplify: f32,
    wet_dry: f32,
}

impl From<&RawParameters> for Parameters {
    fn from(params: &RawParameters) -> Self {
        Parameters {
            wet_dry: params.wet_dry.get(),
            clip_level: ease_in_expo(params.clip_level.get()),
            pre_amplify: ease_in_expo(params.pre_amplify.get()) * 16.0,
            post_amplify: ease_in_expo(params.post_amplify.get()) * 4.0,
        }
    }
}

pub fn ease_in_expo(x: f32) -> f32 {
    if x <= 0.0 {
        0.0
    } else {
        (2.0f32.powf(10.0 * x) - 1.0) / (2.0f32.powf(10.0) - 1.0)
    }
}

/// The raw parameter values that a host DAW will set and modify.
/// These are unscaled and are always in the [0.0, 1.0] range
pub struct RawParameters {
    clip_level: AtomicFloat,
    pre_amplify: AtomicFloat,
    post_amplify: AtomicFloat,
    wet_dry: AtomicFloat,
    /// The host callback, used for communicating with the VST host
    pub host: HostCallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterType {
    PreAmp,
    ClipLevel,
    PostAmp,
    WetDry,
}

fn make_strings(value: f32, label: &str) -> (String, String) {
    (format!("{:.2}", value), label.to_string())
}

macro_rules! table {
    ($macro:ident) => {
        $macro! {
        //  RawParameter identifier, ParameterType identifier
            RawParameters,          ParameterType;
        //  variant                     idx    name            field_name    default    strings
            ParameterType::WetDry,      0,     "Wet/Dry",      wet_dry,      1.0,       |x: f32| make_strings(x * 100.0, "%");
            ParameterType::PreAmp,      1,     "Pre-Amplify",  pre_amplify,  0.7,       |x: f32| make_strings(x, "");
            ParameterType::ClipLevel,   2,     "Clip Level",   clip_level,   0.6,       |x: f32| make_strings(x * 100.0, "%");
            ParameterType::PostAmp,     3,     "Post-Amplify", post_amplify, 0.8,       |x: f32| make_strings(x * 100.0, "% Wet");
        }
    };
}

impl ParameterType {
    pub const COUNT: usize = 4;
}

impl_all! {RawParameters, ParameterType, table}

// Export symbols for main
vst::plugin_main!(Clipper);
