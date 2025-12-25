use gstreamer as gst;

use crate::types::{AudioDriver, OutputTarget};

pub fn create_audio_sink(config: &OutputTarget) -> Result<gst::Element, gst::glib::BoolError> {
    let driver = config.audio_driver.clone().unwrap_or(AudioDriver::Auto);

    match driver {
        #[cfg(target_os = "windows")]
        AudioDriver::Asio => create_asio_sink(config),

        #[cfg(target_os = "windows")]
        AudioDriver::Wasapi => create_wasapi_sink(),

        #[cfg(target_os = "windows")]
        AudioDriver::Auto => {
            // ASIO を優先、なければ WASAPI
            if gst::ElementFactory::find("asiosink").is_some() {
                create_asio_sink(config)
            } else {
                create_wasapi_sink()
            }
        }

        #[cfg(target_os = "macos")]
        AudioDriver::CoreAudio | AudioDriver::Auto => {
            gst::ElementFactory::make("osxaudiosink").build()
        }

        #[cfg(target_os = "linux")]
        AudioDriver::Jack => gst::ElementFactory::make("jackaudiosink").build(),

        #[cfg(target_os = "linux")]
        AudioDriver::Alsa => gst::ElementFactory::make("alsasink").build(),

        #[cfg(target_os = "linux")]
        AudioDriver::Auto => {
            // JACK を優先、なければ ALSA
            if gst::ElementFactory::find("jackaudiosink").is_some() {
                gst::ElementFactory::make("jackaudiosink").build()
            } else {
                gst::ElementFactory::make("alsasink").build()
            }
        }

        #[allow(unreachable_patterns)]
        _ => gst::ElementFactory::make("autoaudiosink").build(),
    }
}

#[cfg(target_os = "windows")]
fn create_asio_sink(config: &OutputTarget) -> Result<gst::Element, gst::glib::BoolError> {
    let mut builder = gst::ElementFactory::make("asiosink");

    if let Some(device) = &config.audio_device {
        builder = builder.property("device-clsid", device);
    }

    if let Some(channels) = &config.audio_channels {
        let ch_str = channels
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(",");
        builder = builder.property("output-channels", &ch_str);
    }

    builder.build()
}

#[cfg(target_os = "windows")]
fn create_wasapi_sink() -> Result<gst::Element, gst::glib::BoolError> {
    gst::ElementFactory::make("wasapisink")
        .property("low-latency", true)
        .build()
}

/// ASIOデバイス一覧を取得
#[cfg(target_os = "windows")]
pub fn list_asio_devices() -> Vec<AsioDevice> {
    let monitor = gst::DeviceMonitor::new();
    monitor.add_filter(Some("Audio/Sink"), None);

    if monitor.start().is_err() {
        return Vec::new();
    }

    let devices = monitor.devices();
    monitor.stop();

    devices
        .iter()
        .filter_map(|d| {
            let props = d.properties()?;
            let api = props.get::<String>("device.api").ok()?;

            if api == "asio" {
                Some(AsioDevice {
                    name: d.display_name().to_string(),
                    clsid: props.get::<String>("device.clsid").ok()?,
                })
            } else {
                None
            }
        })
        .collect()
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AsioDevice {
    pub name: String,
    pub clsid: String,
}

#[cfg(not(target_os = "windows"))]
pub fn list_asio_devices() -> Vec<AsioDevice> {
    Vec::new()
}
