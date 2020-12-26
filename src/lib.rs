#[macro_use]
extern crate vst;

use std::sync::Arc;

use log::{info, LevelFilter};
use median::heap::Filter;
use variant_count::VariantCount;
use vst::{
    api::Supported,
    buffer::AudioBuffer,
    plugin::{CanDo, Category, HostCallback, Info, Plugin, PluginParameters},
    util::AtomicFloat,
};

struct MedianFilter {
    sample_rate: f32,
    params: Arc<RawParameters>,
    left_filter: Filter<f32>,
    right_filter: Filter<f32>,
    last_window_size: usize,
}

impl Plugin for MedianFilter {
    fn new(_: HostCallback) -> Self {
        MedianFilter {
            params: Arc::new(RawParameters {
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn init(&mut self) {
        let result = simple_logging::log_to_file("median_filter.log", LevelFilter::Info);
        if let Err(err) = result {
            println!("Couldn't start logging! {}", err);
        }
        info!("Begin VST log");

        let params = Parameters::from(self.params.as_ref());

        self.last_window_size = params.window_size;
    }

    fn get_info(&self) -> Info {
        Info {
            name: "Median Filter".to_string(),
            vendor: "a2aaron".to_string(),
            // Used by hosts to differentiate between plugins.
            // Don't worry much about this now - just fill in a random number.
            unique_id: 612413,
            version: 1,
            category: Category::Effect,
            // Subtract one here due to "error" type
            parameters: (ParameterType::VARIANT_COUNT - 1) as i32,
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
        self.reset_if_changed();

        let num_samples = buffer.samples();

        let (inputs, mut outputs) = buffer.split();
        let left_input = &inputs[0];
        let left_output = &mut outputs[0];

        for i in 0..num_samples {
            self.left_filter.consume(left_input[i]);
            if self.left_filter.is_empty() != 0 {
                left_output[i] = self.left_filter.median();
            } else {
                left_output[i] = 0.0;
            }
        }

        let right_input = &inputs[1];
        let right_output = &mut outputs[1];

        for i in 0..num_samples {
            self.right_filter.consume(right_input[i]);
            if self.right_filter.is_empty() != 0 {
                right_output[i] = self.right_filter.median();
            } else {
                right_output[i] = 0.0;
            }
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

impl Default for MedianFilter {
    fn default() -> Self {
        MedianFilter {
            sample_rate: 44100.0,
            params: Arc::new(RawParameters::default()),
            left_filter: Filter::new(100),
            right_filter: Filter::new(100),
            last_window_size: 100,
        }
    }
}

impl MedianFilter {
    fn reset_if_changed(&mut self) {
        let params = Parameters::from(self.params.as_ref());
        if params.window_size != self.last_window_size {
            self.left_filter = Filter::new(params.window_size);
            self.right_filter = Filter::new(params.window_size);
            self.last_window_size = params.window_size;
        }
    }
}

struct Parameters {
    window_size: usize,
}

impl From<&RawParameters> for Parameters {
    fn from(params: &RawParameters) -> Self {
        Parameters {
            window_size: ((params.window_size.get() * 100.0) as usize).max(1),
        }
    }
}

/// The raw parameter values that a host DAW will set and modify.
/// These are unscaled and are always in the [0.0, 1.0] range
pub struct RawParameters {
    window_size: AtomicFloat,
}

impl PluginParameters for RawParameters {
    fn get_parameter_label(&self, index: i32) -> String {
        match index.into() {
            ParameterType::WindowSize => "Samples".to_string(),
            ParameterType::Error => "".to_string(),
        }
    }

    fn get_parameter_text(&self, index: i32) -> String {
        let params = Parameters::from(self);
        match index.into() {
            ParameterType::WindowSize => format!("{}", params.window_size),
            ParameterType::Error => "".to_string(),
        }
    }

    fn get_parameter_name(&self, index: i32) -> String {
        match index.into() {
            ParameterType::WindowSize => "Window Size".to_string(),
            ParameterType::Error => "".to_string(),
        }
    }

    fn get_parameter(&self, index: i32) -> f32 {
        match index.into() {
            ParameterType::WindowSize => self.window_size.get(),
            ParameterType::Error => 0.0,
        }
    }

    fn set_parameter(&self, index: i32, value: f32) {
        match index.into() {
            ParameterType::WindowSize => self.window_size.set(value),
            ParameterType::Error => (),
        }
    }

    fn can_be_automated(&self, index: i32) -> bool {
        ParameterType::from(index) != ParameterType::Error
    }

    fn string_to_parameter(&self, _index: i32, _text: String) -> bool {
        false
    }
}

impl Default for RawParameters {
    fn default() -> Self {
        RawParameters {
            window_size: AtomicFloat::new(1.0),
        }
    }
}

/// The type of parameter. "Error" is included as a convience type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, VariantCount)]
pub enum ParameterType {
    WindowSize,
    Error,
}

impl From<i32> for ParameterType {
    fn from(i: i32) -> Self {
        use ParameterType::*;
        match i {
            0 => WindowSize,
            _ => Error,
        }
    }
}

// Export symbols for main
plugin_main!(MedianFilter);
