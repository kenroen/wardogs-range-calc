// Hide the console window on Windows.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::sync::Arc;

mod app;
mod chrome;
mod clipboard;
mod coords;
mod theme;

fn main() -> eframe::Result {
    let viewport = egui::ViewportBuilder::default()
        .with_title("WARDOGS // Artillery Range")
        .with_app_id("wardogs-range-calc")
        .with_inner_size([560.0, 760.0])
        .with_min_inner_size([480.0, 640.0]);
    let options = eframe::NativeOptions {
        viewport,
        persist_window: true,
        wgpu_options: wgpu_options(),
        ..Default::default()
    };
    eframe::run_native("WARDOGS // Artillery Range", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}

#[cfg(windows)]
fn wgpu_options() -> eframe::WgpuConfiguration {
    use eframe::wgpu::{Backend, Backends, DeviceType, Dx12Compiler, Dx12SwapchainKind, PowerPreference};
    let mut config = eframe::WgpuConfiguration::default();
    if let eframe::egui_wgpu::WgpuSetup::CreateNew(setup) = &mut config.wgpu_setup {
        setup.instance_descriptor.backend_options.dx12.shader_compiler =
            Dx12Compiler::from_env().unwrap_or(Dx12Compiler::Fxc);
        setup.instance_descriptor.backend_options.dx12.presentation_system =
            Dx12SwapchainKind::from_env().unwrap_or(Dx12SwapchainKind::DxgiFromVisual);
        if let Some(backends) = Backends::from_env() {
            setup.instance_descriptor.backends = backends;
        } else {
            setup.instance_descriptor.backends = Backends::DX12 | Backends::GL;
            let integrated = setup.power_preference == PowerPreference::LowPower;
            setup.native_adapter_selector = Some(Arc::new(move |adapters, surface| {
                adapters
                    .iter()
                    .filter(|a| surface.is_none_or(|s| a.is_surface_supported(s)))
                    .min_by_key(|a| {
                        let info = a.get_info();
                        let device = match info.device_type {
                            DeviceType::DiscreteGpu => 1 + u8::from(integrated),
                            DeviceType::IntegratedGpu => 2 - u8::from(integrated),
                            DeviceType::Other => 3,
                            DeviceType::VirtualGpu => 4,
                            DeviceType::Cpu => 5,
                        };
                        (info.backend != Backend::Dx12, device)
                    })
                    .cloned()
                    .ok_or_else(|| "no graphics adapter can draw to the window".to_owned())
            }));
        }
    }
    config
}

#[cfg(not(windows))]
fn wgpu_options() -> eframe::WgpuConfiguration {
    eframe::WgpuConfiguration::default()
}
