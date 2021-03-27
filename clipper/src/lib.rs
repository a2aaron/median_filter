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
    sample_rate: f32,
    params: Arc<RawParameters2>,
}

impl Plugin for Clipper {
    fn new(host: HostCallback) -> Self {
        Clipper {
            params: Arc::new(RawParameters2::default(host)),
            sample_rate: 44100.0,
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
            parameters: ParameterType2::COUNT as i32,
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

    fn set_sample_rate(&mut self, rate: f32) {
        self.sample_rate = rate;
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

impl From<&RawParameters2> for Parameters {
    fn from(params: &RawParameters2) -> Self {
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
pub struct RawParameters2 {
    clip_level: AtomicFloat,
    pre_amplify: AtomicFloat,
    post_amplify: AtomicFloat,
    wet_dry: AtomicFloat,
    /// The host callback, used for communicating with the VST host
    pub host: HostCallback,
}

impl RawParameters2 {
    pub fn set(&self, value: f32, parameter: ParameterType2) {
        // These are needed so Ableton will notice parameter changes in the
        // "Configure" window.
        // TODO: investigate if I should send this only on mouseup/mousedown
        self.host.begin_edit(parameter.into());
        self.get_ref(parameter).set(value);
        self.host.end_edit(parameter.into());
    }

    pub fn get(&self, parameter: ParameterType2) -> f32 {
        self.get_ref(parameter).get()
    }

    /// Returns a user-facing text output for the given parameter. This is broken
    /// into a tuple consisting of (`value`, `units`)
    fn get_strings(&self, parameter: ParameterType2) -> (String, String) {
        let params = Parameters::from(self);

        fn make_strings(value: f32, label: &str) -> (String, String) {
            (format!("{:.2}", value), label.to_string())
        }

        match parameter {
            ParameterType2::PreAmp => make_strings(params.pre_amplify * 100.0, "%"),
            ParameterType2::ClipLevel => make_strings(params.clip_level, ""),
            ParameterType2::PostAmp => make_strings(params.post_amplify * 100.0, "%"),
            ParameterType2::WetDry => make_strings(params.wet_dry * 100.0, "% Wet"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterType2 {
    PreAmp,
    ClipLevel,
    PostAmp,
    WetDry,
}

macro_rules! table {
    ($macro:ident) => {
        $macro! {
        //  RawParameter identifier, ParameterType identifier
            RawParameters2,          ParameterType2;
        //  variant                     idx    name            field_name
            ParameterType2::WetDry,      0,     "Wet/Dry",      wet_dry,      1.0;
            ParameterType2::PreAmp,      1,     "Pre-Amplify",  pre_amplify,  0.7;
            ParameterType2::ClipLevel,   2,     "Clip Level",   clip_level,   0.6;
            ParameterType2::PostAmp,     3,     "Post-Amplify", post_amplify, 0.8;
        }
    };
}

impl ParameterType2 {
    pub const COUNT: usize = 4;
}

impl_all! {RawParameters2, ParameterType2, table}

// Export symbols for main
vst::plugin_main!(Clipper);
