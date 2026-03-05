use windows::{
    core::HRESULT,
    Win32::{
        Foundation::RPC_E_CHANGED_MODE,
        Media::Audio::{eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator},
        Media::Audio::Endpoints::IAudioEndpointVolume,
        System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
    },
};

pub fn is_system_muted() -> Result<bool, String> {
    let endpoint = endpoint_volume()?;
    let muted = unsafe { endpoint.GetMute() }
        .map_err(|error| format!("failed to get mute state: {error}"))?;
    Ok(muted.as_bool())
}

pub fn set_system_mute(mute: bool) -> Result<(), String> {
    let endpoint = endpoint_volume()?;
    unsafe { endpoint.SetMute(mute, std::ptr::null()) }
        .map_err(|error| format!("failed to set mute state: {error}"))?;
    Ok(())
}

fn endpoint_volume() -> Result<IAudioEndpointVolume, String> {
    initialize_com()?;

    let enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
            .map_err(|error| format!("failed to create audio device enumerator: {error}"))?;

    let device = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) }
        .map_err(|error| format!("failed to get default output device: {error}"))?;

    let endpoint = unsafe { device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) }
        .map_err(|error| format!("failed to activate endpoint volume: {error}"))?;

    Ok(endpoint)
}

fn initialize_com() -> Result<(), String> {
    let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    if result.is_ok() || result == RPC_E_CHANGED_MODE {
        return Ok(());
    }

    Err(format!(
        "failed to initialize COM for audio operations: {}",
        format_hresult(result)
    ))
}

fn format_hresult(hr: HRESULT) -> String {
    format!("0x{:08X}", hr.0 as u32)
}
