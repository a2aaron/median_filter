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
#[macro_export]
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

#[macro_export]
macro_rules! impl_display {
     ($($variant:pat, $idx:expr, $name:expr, $_:expr, $_default:expr;)*) => {
        impl std::fmt::Display for ParameterType {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $($variant => write!(f, $name),)*
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_from_i32 {
    ($($variant:expr, $idx:expr, $_name:expr, $_field_name:expr, $_default:expr;)*) => {
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

#[macro_export]
macro_rules! impl_into_i32 {
    ($($variant:pat, $idx:expr, $_name:expr, $_field_name:expr, $_default:expr;)*) => {
        impl From<ParameterType> for i32 {
            fn from(x: ParameterType) -> i32 {
                match x {
                    $($variant => $idx,)*
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_get_ref {
    ($($variant:pat, $_:expr, $_name:expr, $field_name:ident, $_default:expr;)*) => {
        impl RawParameters {
            fn get_ref(&self, x: ParameterType) -> &AtomicFloat {
                match x {
                    $($variant => &self.$field_name,)*
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_get_default {
    ($($variant:pat, $_:expr, $_name:expr, $_field_name:ident, $default:expr;)*) => {
        impl RawParameters {
            fn get_default(x: ParameterType) -> f32 {
                match x {
                    $($variant => $default,)*
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_default {
    ($($variant:pat, $_:expr, $_name:expr, $field_name:ident, $default:expr;)*) => {
        impl RawParameters {
            fn default(host: HostCallback) -> Self {
                RawParameters {
                    $($field_name: AtomicFloat::new($default),)*
                    host,
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_all {
    ($raw_parameters: ident, $parameter_type: ident, $table: ident) => {
        impl_plugin_parameters! {$raw_parameters, $parameter_type}
        $table! {impl_from_i32}
        $table! {impl_into_i32}
        $table! {impl_display}
        $table! {impl_get_ref}
        $table! {impl_default}
        $table! {impl_get_default}
    };
}
