#[macro_use]
extern crate vst;

use std::convert::TryFrom;
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
    params: Arc<RawParameters>,
}

impl Plugin for Clipper {
    fn new(host: HostCallback) -> Self {
        Clipper {
            params: Arc::new(RawParameters::default(host)),
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

impl RawParameters {
    pub fn get_ref(&self, parameter: ParameterType) -> &AtomicFloat {
        match parameter {
            ParameterType::PreAmp => &self.pre_amplify,
            ParameterType::ClipLevel => &self.clip_level,
            ParameterType::PostAmp => &self.post_amplify,
            ParameterType::WetDry => &self.wet_dry,
        }
    }

    pub fn set(&self, value: f32, parameter: ParameterType) {
        // These are needed so Ableton will notice parameter changes in the
        // "Configure" window.
        // TODO: investigate if I should send this only on mouseup/mousedown
        self.host.begin_edit(parameter.into());
        self.get_ref(parameter).set(value);
        self.host.end_edit(parameter.into());
    }

    pub fn get(&self, parameter: ParameterType) -> f32 {
        self.get_ref(parameter).get()
    }

    /// Returns a user-facing text output for the given parameter. This is broken
    /// into a tuple consisting of (`value`, `units`)
    fn get_strings(&self, parameter: ParameterType) -> (String, String) {
        let params = Parameters::from(self);

        fn make_strings(value: f32, label: &str) -> (String, String) {
            (format!("{:.2}", value), label.to_string())
        }

        match parameter {
            ParameterType::PreAmp => make_strings(params.pre_amplify * 100.0, "%"),
            ParameterType::ClipLevel => make_strings(params.clip_level, ""),
            ParameterType::PostAmp => make_strings(params.post_amplify * 100.0, "%"),
            ParameterType::WetDry => make_strings(params.wet_dry * 100.0, "% Wet"),
        }
    }

    fn default(host: HostCallback) -> Self {
        RawParameters {
            clip_level: AtomicFloat::new(0.6),   // Clip at 0.06ish
            pre_amplify: AtomicFloat::new(0.7),  // 200%
            post_amplify: AtomicFloat::new(0.8), // 100%
            wet_dry: AtomicFloat::new(1.0),      // 100% wet
            host,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterType {
    PreAmp,
    ClipLevel,
    PostAmp,
    WetDry,
}

macro_rules! table {
    ($macro:ident) => {
        $macro! {
            //  variant                     idx    name
            ParameterType::WetDry,      0,     "Wet/Dry";
            ParameterType::PreAmp,      1,     "Pre-Amplify";
            ParameterType::ClipLevel,   2,     "Clip Level";
            ParameterType::PostAmp,     3,     "Post-Amplify";
        }
    };
}

impl ParameterType {
    pub const COUNT: usize = 4;
}

/// Implement PluginParameters for `$raw_parameters`. `$parameter_type` must
/// be an enum which implements `TryFrom<i32>` and `Display`
/// `$raw_parameters` must implement the following functions
/// get(&self, $parameter_type) -> f32
///     returns the normalized f32 value of the given parameter
/// set(&mut self, $parameter_type, value: f32)
///     sets the normalized f32 value of the given parameter
/// get_strings(&self, $parameter_type) -> (String, String)
///     returns a tuple where the first String is the parameter's name
///     (ex: "Master Volume") and the second tuple is the parameter's value
///     (ex: "12 db")
macro_rules! impl_plugin_parameters {
    ($raw_parameters: ident, $parameter_type: ident) => {
        impl PluginParameters for $raw_parameters {
            fn get_parameter_label(&self, index: i32) -> String {
                if let Ok(parameter) = $parameter_type::try_from(index) {
                    self.get_strings(parameter).1
                } else {
                    "".to_string()
                }
            }

            fn get_parameter_text(&self, index: i32) -> String {
                if let Ok(parameter) = $parameter_type::try_from(index) {
                    self.get_strings(parameter).0
                } else {
                    "".to_string()
                }
            }

            fn get_parameter_name(&self, index: i32) -> String {
                if let Ok(param) = $parameter_type::try_from(index) {
                    param.to_string()
                } else {
                    "".to_string()
                }
            }

            fn get_parameter(&self, index: i32) -> f32 {
                if let Ok(parameter) = $parameter_type::try_from(index) {
                    self.get(parameter)
                } else {
                    0.0
                }
            }

            fn set_parameter(&self, index: i32, value: f32) {
                if let Ok(parameter) = $parameter_type::try_from(index) {
                    // This is needed because some VST hosts, such as Ableton, echo a
                    // parameter change back to the plugin. This causes issues such as
                    // weird knob behavior where the knob "flickers" because the user tries
                    // to change the knob value, but ableton keeps sending back old, echoed
                    // values.
                    #[allow(clippy::float_cmp)]
                    if self.get(parameter) == value {
                        return;
                    }

                    self.set(value, parameter);
                }
            }

            fn can_be_automated(&self, index: i32) -> bool {
                $parameter_type::try_from(index).is_ok()
            }

            fn string_to_parameter(&self, _index: i32, _text: String) -> bool {
                false
            }
        }
    };
}

macro_rules! impl_display {
     ($($variant:pat, $idx:expr, $name:expr;)*) => {
        impl std::fmt::Display for ParameterType {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $($variant => write!(f, $name),)*
                }
            }
        }
    };
}

macro_rules! impl_from_i32 {
    ($($variant:expr, $idx:expr, $_:expr;)*) => {
        impl TryFrom<i32> for ParameterType {
            type Error = ();
            fn try_from(x: i32) -> Result<Self, Self::Error> {
                match x {
                    $($idx => Ok($variant),)*
                    _ => Err(()),
                }
            }
        }
    }
}

macro_rules! impl_into_i32 {
    ($($variant:pat, $idx:expr, $_:expr;)*) => {
        impl From<ParameterType> for i32 {
            fn from(x: ParameterType) -> i32 {
                match x {
                    $($variant => $idx,)*
                }
            }
        }
    };
}

table! {impl_from_i32}
table! {impl_into_i32}
table! {impl_display}

impl_plugin_parameters! {RawParameters, ParameterType}

// Export symbols for main
plugin_main!(Clipper);
