#![cfg_attr(feature = "simd", feature(portable_simd))]

use nih_plug::prelude::*;

use std::sync::Arc;

pub mod adaa;

/// The number of channels this plugin supports. Hard capped at 2 for now (SIMD later?)
pub const NUM_CHANNELS: u32 = 2;

struct RustAdaa {
    params: Arc<RustAdaaParams>,
    buffer_config: BufferConfig,
    adaa_l: adaa::Adaa2,
    adaa_r: adaa::Adaa2
}

#[derive(Params)]
struct RustAdaaParams {
    /// Pre-gain (helpful for input trimming when e.g. main gain is automated)
    #[id = "pre_gain"]
    pub pre_gain: FloatParam,

    /// Main gain (gain right before drive - this is the one you should automate etc)
    #[id = "main_gain"]
    pub main_gain: FloatParam,

    /// Post-gain (output trimming after nonlinearity)
    #[id = "post_gain"]
    pub post_gain: FloatParam,

    /// Nonlinearity function
    #[id = "nl_function"]
    pub nl_function: EnumParam<NlFunctionParam>,
}

#[derive(Enum, PartialEq)]
enum NlFunctionParam {
    #[id = "hard-clip"]
    HardClip,

    #[id = "tanh"]
    Tanh,
}

impl RustAdaaParams {
    fn new() -> Self {
        let gain_range = FloatRange::Linear {
            min: util::db_to_gain(-16.0),
            max: util::db_to_gain( 16.0)
        };

        // Smooth to target logarithmically (there is no zero as we work in volt gain) in 10ms
        let smoothing_style = SmoothingStyle::Logarithmic(10.0);

        let db_to_string = formatters::v2s_f32_gain_to_db(2); // 2 digits of precision
        let string_to_db = formatters::s2v_f32_gain_to_db();

        Self {
            pre_gain: FloatParam::new("Pre Gain", 1.0, gain_range)
                .with_smoother(smoothing_style)
                .with_value_to_string(db_to_string.clone())
                .with_string_to_value(string_to_db.clone()),
            main_gain: FloatParam::new("Main Gain", 1.0, gain_range)
                .with_smoother(smoothing_style)
                .with_value_to_string(db_to_string.clone())
                .with_string_to_value(string_to_db.clone()),
            post_gain: FloatParam::new("Post Gain", 1.0, gain_range)
                .with_smoother(smoothing_style)
                .with_value_to_string(db_to_string.clone())
                .with_string_to_value(string_to_db.clone()),
            nl_function: EnumParam::new("Function", NlFunctionParam::HardClip),
        }
    }
}

impl Default for RustAdaa {
    fn default() -> Self {
        Self {
            params: Arc::new(RustAdaaParams::new()),

            buffer_config: BufferConfig {
                sample_rate: 1.0,
                min_buffer_size: None,
                max_buffer_size: 0,
                process_mode: ProcessMode::Realtime,
            },
            adaa_l: adaa::Adaa2::default(),
            adaa_r: adaa::Adaa2::default()
        }
    }
}

impl Plugin for RustAdaa {
    const NAME: &'static str = "RustAdaa";
    const VENDOR: &'static str = "selenologist";
    const URL: &'static str = "https://github.com/selenologist/rustadaa";
    const EMAIL: &'static str = "none@example.com";

    const VERSION: &'static str = "0.1.0";

    const DEFAULT_NUM_INPUTS: u32 = NUM_CHANNELS;
    const DEFAULT_NUM_OUTPUTS: u32 = NUM_CHANNELS;

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn accepts_bus_config(&self, config: &BusConfig) -> bool {
        // Only do stereo
        config.num_input_channels == NUM_CHANNELS && config.num_output_channels == NUM_CHANNELS
    }

    fn initialize(
        &mut self,
        _bus_config: &BusConfig,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext,
    ) -> bool {
        self.buffer_config = *buffer_config;

        true
    }

    fn reset(&mut self) {}

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext,
    ) -> ProcessStatus {        
        // until I remember how to use traits properly
        let shaper = match self.params.nl_function.value() {
            NlFunctionParam::HardClip => |adaa: &mut adaa::Adaa2, x| adaa.process::<adaa::HardClip>(x),
            NlFunctionParam::Tanh => |adaa: &mut adaa::Adaa2, x| adaa.process::<adaa::Tanh>(x),
        };

        for mut channel_samples in buffer.iter_samples() {
            let xpre_gain = self.params.pre_gain.smoothed.next();
            let main_gain = self.params.main_gain.smoothed.next();
            let post_gain = self.params.post_gain.smoothed.next();

            // pre-gain and main gain are actually applied at the same time.
            // it's cheaper to premultiply the gains so only one multiply is needed per sample.
            let pre_gain = xpre_gain * main_gain;

            for (sample, adaa) in channel_samples.iter_mut().zip([&mut self.adaa_l, &mut self.adaa_r]) {
                *sample = shaper(adaa, (*sample * pre_gain) as f64) as f32 * post_gain;
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for RustAdaa {
    const CLAP_ID: &'static str = "com.lunarsynth.rustadaa";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Rust port of jatinchowdhury18's ADAA plugin");
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Distortion,
        ClapFeature::Utility,
    ];
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
}

impl Vst3Plugin for RustAdaa {
    const VST3_CLASS_ID: [u8; 16] = *b"RustAdaaPlugLuna";
    const VST3_CATEGORIES: &'static str = "Fx|Distortion";
}

nih_export_clap!(RustAdaa);
nih_export_vst3!(RustAdaa);
