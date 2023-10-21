use atomic_f64::AtomicF64;
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
    events::event_types::ParamValueEvent,
    plugin::descriptor::features::{STEREO, SYNTHESIZER},
    prelude::*,
    process::audio::PairedChannels,
    utils::Cookie,
};
use num::Float;
use std::{f64::consts::PI, ffi::CStr, num::Wrapping, sync::atomic::Ordering};

mod atomic_f64;

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
    shared: &'a PicosineShared<'a>,
    rate_hz: f64,
    gain_factor: f64,
    time_sc: Wrapping<u32>,
}

impl<'a> PicosineAudioProcessor<'a> {
    fn write_signal<S: Float>(&self, output: &mut [S], freq_hz: f64) {
        for (idx, output) in output.iter_mut().enumerate() {
            *output = S::from(
                f64::sin(
                    2.0f64 * PI * freq_hz * (self.time_sc.0 as usize + idx) as f64 / self.rate_hz,
                ) * self.gain_factor,
            )
            .unwrap()
        }
    }

    fn process_channel_pairs<T: Float>(&self, channel_pairs: PairedChannels<T>, freq_hz: f64) {
        for channel_pair in channel_pairs {
            match channel_pair {
                ChannelPair::InputOnly(_) => {}
                ChannelPair::OutputOnly(output) => self.write_signal(output, freq_hz),
                ChannelPair::InputOutput(_input, output) => self.write_signal(output, freq_hz),
                ChannelPair::InPlace(buf) => self.write_signal(buf, freq_hz),
            }
        }
    }

    fn handle_events(&mut self, events: Events) {
        let input_param_value_events = events
            .input
            .into_iter()
            .filter_map(|event| event.as_event::<ParamValueEvent>());

        for input_param_value_event in input_param_value_events {
            match (
                input_param_value_event.param_id(),
                input_param_value_event.value(),
            ) {
                (0, new_freq_hz) => self.shared.freq_hz.store(new_freq_hz, Ordering::Relaxed),
                (_, _new_value) => panic!("unknown input parameter id"),
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
        shared: &'a PicosineShared,
        audio_config: AudioConfiguration,
    ) -> Result<Self, PluginError> {
        Ok(Self {
            shared: shared,
            gain_factor: 0.5,
            rate_hz: audio_config.sample_rate,
            time_sc: Wrapping(0),
            _host: host,
        })
    }

    fn process(
        &mut self,
        _process: Process,
        mut audio: Audio,
        events: Events,
    ) -> Result<ProcessStatus, PluginError> {
        self.handle_events(events);

        let freq_hz = self.shared.freq_hz.load(Ordering::Relaxed);

        for mut port_pair in &mut audio {
            match port_pair.channels().unwrap() {
                SampleType::Both(_f32_channel_pairs, f64_channel_pairs) => {
                    self.process_channel_pairs(f64_channel_pairs, freq_hz)
                }
                SampleType::F32(f32_channel_pairs) => {
                    self.process_channel_pairs(f32_channel_pairs, freq_hz)
                }
                SampleType::F64(f64_channel_pairs) => {
                    self.process_channel_pairs(f64_channel_pairs, freq_hz)
                }
            };
        }

        self.time_sc += audio.frames_count();

        Ok(ProcessStatus::Continue)
    }
}

pub struct PicosineShared<'a> {
    _host: HostHandle<'a>,
    freq_hz: AtomicF64,
}

impl<'a> PluginShared<'a> for PicosineShared<'a> {
    fn new(host: HostHandle<'a>) -> Result<Self, PluginError> {
        Ok(Self {
            _host: host,
            freq_hz: AtomicF64::new(440.0),
        })
    }
}

pub struct PicosineMainThread<'a> {
    shared: &'a PicosineShared<'a>,
    _host: HostMainThreadHandle<'a>,
}

impl<'a> PluginMainThread<'a, PicosineShared<'a>> for PicosineMainThread<'a> {
    fn new(
        host: HostMainThreadHandle<'a>,
        shared: &'a PicosineShared,
    ) -> Result<Self, PluginError> {
        Ok(Self {
            shared: shared,
            _host: host,
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
        // Not sure what flush does, ignoring for now. ~ Stef 21 okt 2023.
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
            Some(self.shared.freq_hz.load(Ordering::Relaxed))
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
        // Not sure what flush does, ignoring for now. ~ Stef 21 okt 2023.
    }
}

clack_export_entry!(SinglePluginEntry<Picosine>);
