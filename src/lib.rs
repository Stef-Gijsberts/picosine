use clack_extensions::audio_ports::{
    AudioPortFlags, AudioPortInfoData, AudioPortInfoWriter, AudioPortType, PluginAudioPorts,
    PluginAudioPortsImpl,
};
use clack_plugin::{
    plugin::descriptor::features::{STEREO, SYNTHESIZER},
    prelude::*,
    process::audio::PairedChannels,
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
        builder.register::<PluginAudioPorts>();
    }
}

pub struct PicosineAudioProcessor<'a> {
    _host: HostAudioThreadHandle<'a>,
    freq: f64,
    rate: f64,
    gain: f64,
    t: Wrapping<u32>,
}

impl<'a> PicosineAudioProcessor<'a> {
    fn write_signal<S: Float>(&self, output: &mut [S]) {
        for (idx, output) in output.iter_mut().enumerate() {
            *output = S::from(
                f64::sin(2.0f64 * PI * self.freq * (self.t.0 as usize + idx) as f64 / self.rate)
                    * self.gain,
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
            gain: 0.5,
            freq: 440.0,
            rate: audio_config.sample_rate,
            t: Wrapping(0),
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

        self.t += audio.frames_count();

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
}

impl<'a> PluginMainThread<'a, PicosineShared<'a>> for PicosineMainThread<'a> {
    fn new(
        host: HostMainThreadHandle<'a>,
        shared: &'a PicosineShared,
    ) -> Result<Self, PluginError> {
        Ok(Self {
            _shared: shared,
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

clack_export_entry!(SinglePluginEntry<Picosine>);
