use clack_extensions::{
    audio_ports::{
        AudioPortFlags, AudioPortInfoData, AudioPortInfoWriter, AudioPortType, PluginAudioPorts,
        PluginAudioPortsImpl,
    },
    params::{
        implementation::{
            ParamDisplayWriter, ParamInfoWriter, PluginAudioProcessorParams, PluginMainThreadParams,
        },
        info::{ParamInfoData, ParamInfoFlags},
        PluginParams,
    },
};
use clack_plugin::{
    plugin::descriptor::features::{STEREO, SYNTHESIZER},
    prelude::*,
    process::audio::PairedChannels,
    utils::Cookie,
};
use num::Float;
use std::{f64::consts::PI, ffi::CStr, num::Wrapping};

pub struct Picosine;

impl Plugin for Picosine {
    type AudioProcessor<'a> = PicosineAudioProcessor<'a>;

    type Shared<'a> = PicosineShared<'a>;
    type MainThread<'a> = PicosineMainThread<'a>;

    fn get_descriptor() -> Box<dyn PluginDescriptor> {
        Box::new(StaticPluginDescriptor {
            id: CStr::from_bytes_with_nul(b"link.stef.audio.picosine\0").unwrap(),
            name: CStr::from_bytes_with_nul(b"PicoSine\0").unwrap(),
            features: Some(&[SYNTHESIZER, STEREO]),
            ..Default::default()
        })
    }

    fn declare_extensions(builder: &mut PluginExtensions<Self>, _shared: &PicosineShared) {
        builder
            .register::<PluginAudioPorts>()
            .register::<PluginParams>();
    }
}

pub struct PicosineAudioProcessor<'a> {
    _host: HostAudioThreadHandle<'a>,
    freq_hz: f64,
    rate_hz: f64,
    gain_factor: f64,
    time_sc: Wrapping<u32>,
}

impl<'a> PicosineAudioProcessor<'a> {
    fn write_signal<S: Float>(&self, output: &mut [S]) {
        for (idx, output) in output.iter_mut().enumerate() {
            *output = S::from(
                f64::sin(
                    2.0f64 * PI * self.freq_hz * (self.time_sc.0 as usize + idx) as f64
                        / self.rate_hz,
                ) * self.gain_factor,
            )
            .unwrap()
        }
    }

    fn process_channel_pairs<T: Float>(&self, channel_pairs: PairedChannels<T>) {
        for channel_pair in channel_pairs {
            match channel_pair {
                ChannelPair::InputOnly(_) => {}
                ChannelPair::OutputOnly(output) => self.write_signal(output),
                ChannelPair::InputOutput(_input, output) => self.write_signal(output),
                ChannelPair::InPlace(buf) => self.write_signal(buf),
            }
        }
    }
}

impl<'a> PluginAudioProcessor<'a, PicosineShared<'a>, PicosineMainThread<'a>>
    for PicosineAudioProcessor<'a>
{
    fn activate(
        host: HostAudioThreadHandle<'a>,
        _main_thread: &mut PicosineMainThread,
        _shared: &'a PicosineShared,
        audio_config: AudioConfiguration,
    ) -> Result<Self, PluginError> {
        Ok(Self {
            gain_factor: 0.5,
            freq_hz: 440.0,
            rate_hz: audio_config.sample_rate,
            time_sc: Wrapping(0),
            _host: host,
        })
    }

    fn process(
        &mut self,
        _process: Process,
        mut audio: Audio,
        _events: Events,
    ) -> Result<ProcessStatus, PluginError> {
        for mut port_pair in &mut audio {
            match port_pair.channels().unwrap() {
                SampleType::Both(_f32_channel_pairs, f64_channel_pairs) => {
                    self.process_channel_pairs(f64_channel_pairs)
                }
                SampleType::F32(f32_channel_pairs) => self.process_channel_pairs(f32_channel_pairs),
                SampleType::F64(f64_channel_pairs) => self.process_channel_pairs(f64_channel_pairs),
            };
        }

        self.time_sc += audio.frames_count();

        Ok(ProcessStatus::Continue)
    }
}

pub struct PicosineShared<'a> {
    _host: HostHandle<'a>,
}

impl<'a> PluginShared<'a> for PicosineShared<'a> {
    fn new(host: HostHandle<'a>) -> Result<Self, PluginError> {
        Ok(Self { _host: host })
    }
}

pub struct PicosineMainThread<'a> {
    _shared: &'a PicosineShared<'a>,
    _host: HostMainThreadHandle<'a>,
    freq_hz: f64,
}

impl<'a> PluginMainThread<'a, PicosineShared<'a>> for PicosineMainThread<'a> {
    fn new(
        host: HostMainThreadHandle<'a>,
        shared: &'a PicosineShared,
    ) -> Result<Self, PluginError> {
        Ok(Self {
            _shared: shared,
            _host: host,
            freq_hz: 440.0,
        })
    }
}

impl<'a> PluginAudioPortsImpl for PicosineMainThread<'a> {
    fn count(&self, _is_input: bool) -> u32 {
        1
    }

    fn get(&self, _is_input: bool, index: u32, writer: &mut AudioPortInfoWriter) {
        if index == 0 {
            writer.set(&AudioPortInfoData {
                id: 0,
                name: b"main",
                channel_count: 2,
                flags: AudioPortFlags::IS_MAIN,
                port_type: Some(AudioPortType::STEREO),
                in_place_pair: None,
            });
        }
    }
}

impl<'a> PluginAudioProcessorParams for PicosineAudioProcessor<'a> {
    fn flush(
        &mut self,
        _input_parameter_changes: &InputEvents,
        _output_parameter_changes: &mut OutputEvents,
    ) {
    }
}

impl<'a> PluginMainThreadParams for PicosineMainThread<'a> {
    fn count(&self) -> u32 {
        1
    }

    fn get_info(&self, param_index: u32, info: &mut ParamInfoWriter) {
        if param_index > 0 {
            return;
        }

        info.set(&ParamInfoData {
            id: 0,
            name: "Frequency",
            module: "picosine/frequency",
            default_value: 440.0,
            min_value: 30.0,
            max_value: 1000.0,
            flags: ParamInfoFlags::IS_STEPPED,
            cookie: Cookie::empty(),
        })
    }

    fn get_value(&self, param_id: u32) -> Option<f64> {
        if param_id == 0 {
            Some(self.freq_hz as f64)
        } else {
            None
        }
    }

    fn value_to_text(
        &self,
        param_id: u32,
        value: f64,
        writer: &mut ParamDisplayWriter,
    ) -> core::fmt::Result {
        use ::core::fmt::Write;
        println!("Format param {param_id}, value {value}");

        if param_id == 0 {
            write!(writer, "{} hz", value as u32)
        } else {
            Ok(())
        }
    }

    fn text_to_value(&self, _param_id: u32, _text: &str) -> Option<f64> {
        None
    }

    fn flush(&mut self, _input_events: &InputEvents, _output_events: &mut OutputEvents) {
        // let value_events = input_events.iter().filter_map(|e| match e.as_event()? {
        //     Event::ParamValue(v) => Some(v),
        //     _ => None,
        // });

        // for value in value_events {
        //     if value.param_id() == 0 {
        //         self.freq_hz = value.value() as u32;
        //     }
        // }
    }
}

clack_export_entry!(SinglePluginEntry<Picosine>);
