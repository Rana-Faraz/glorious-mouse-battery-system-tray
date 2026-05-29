use crate::battery::{BatterySnapshot, MouseBattery};
use crate::config::{AppConfig, ConfigStore};
use crate::{startup, tray};
use single_instance::SingleInstance;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tray_icon::{menu::MenuEvent, TrayIconEvent};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::WindowId;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let instance = SingleInstance::new("model-d2-pro-battery")?;
    if !instance.is_single() {
        tracing::info!("another instance is already running; exiting");
        return Ok(());
    }

    let config_store = ConfigStore::new()?;
    let config = config_store.load();
    tracing::info!(config_path = %config_store.path().display(), "starting native tray app");

    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    TrayIconEvent::set_event_handler(Some({
        let proxy = proxy.clone();
        move |event| {
            let _ = proxy.send_event(UserEvent::Tray(event));
        }
    }));
    MenuEvent::set_event_handler(Some({
        let proxy = proxy.clone();
        move |event| {
            let _ = proxy.send_event(UserEvent::Menu(event));
        }
    }));

    let (worker_tx, worker_rx) = mpsc::channel();
    spawn_worker(proxy, worker_rx, config.clone());

    let mut app = NativeApp {
        worker_tx,
        config_store,
        config,
        tray: None,
        latest_snapshot: None,
        status_override: None,
        autostart_enabled: startup::is_enabled().unwrap_or(false),
    };

    event_loop.run_app(&mut app)?;
    Ok(())
}

#[derive(Debug)]
enum UserEvent {
    Tray(TrayIconEvent),
    Menu(MenuEvent),
    Worker(WorkerEvent),
}

#[derive(Debug)]
enum WorkerCommand {
    Refresh,
    SelectDevice(Option<String>),
    Firmware,
    Stop,
}

#[derive(Debug)]
enum WorkerEvent {
    Snapshot(BatterySnapshot),
    FirmwareResult(String),
}

struct NativeApp {
    worker_tx: mpsc::Sender<WorkerCommand>,
    config_store: ConfigStore,
    config: AppConfig,
    tray: Option<tray::Tray>,
    latest_snapshot: Option<BatterySnapshot>,
    status_override: Option<String>,
    autostart_enabled: bool,
}

impl ApplicationHandler<UserEvent> for NativeApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
        self.request_refresh();
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Tray(_event) => {}
            UserEvent::Menu(event) => self.handle_menu(event, event_loop),
            UserEvent::Worker(event) => self.handle_worker_event(event),
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _event: WindowEvent,
    ) {
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        let _ = self.worker_tx.send(WorkerCommand::Stop);
    }
}

impl NativeApp {
    fn handle_menu(&mut self, event: MenuEvent, event_loop: &ActiveEventLoop) {
        let id = tray::menu_id(&event);

        if id == tray::MENU_REFRESH {
            self.status_override = None;
            self.request_refresh();
            return;
        }

        if id == tray::MENU_FIRMWARE {
            let _ = self.worker_tx.send(WorkerCommand::Firmware);
            return;
        }

        if id == tray::MENU_AUTOSTART {
            let next = !self.autostart_enabled;
            match startup::set_enabled(next) {
                Ok(()) => {
                    self.autostart_enabled = next;
                    self.update_tray();
                }
                Err(error) => {
                    tracing::warn!(%error, "failed to toggle startup");
                    self.status_override = Some(format!("Failed to update startup: {error}"));
                    self.update_tray();
                }
            }
            return;
        }

        if id == tray::MENU_EXIT {
            let _ = self.worker_tx.send(WorkerCommand::Stop);
            event_loop.exit();
            return;
        }

        if let Some(device_key) = tray::device_key_from_menu_id(id) {
            let selected = Some(device_key.to_string());
            self.config.selected_device_key = selected.clone();
            if let Err(error) = self.config_store.save(&self.config) {
                tracing::warn!(%error, "failed to save selected device");
            }
            let _ = self.worker_tx.send(WorkerCommand::SelectDevice(selected));
        }
    }

    fn handle_worker_event(&mut self, event: WorkerEvent) {
        match event {
            WorkerEvent::Snapshot(snapshot) => {
                if matches!(
                    self.config.selected_device_key.as_ref(),
                    Some(configured) if snapshot.selected_key.as_deref() != Some(configured.as_str())
                ) {
                    self.config.selected_device_key = snapshot.selected_key.clone();
                    if let Err(error) = self.config_store.save(&self.config) {
                        tracing::warn!(%error, "failed to persist selected device fallback");
                    }
                }

                if let crate::battery::BatteryStatus::Unknown {
                    raw_status,
                    raw_battery,
                    ..
                } = snapshot.status
                {
                    tracing::warn!(raw_status, raw_battery, "unknown HID battery status");
                }

                self.latest_snapshot = Some(snapshot);
                self.status_override = None;
                self.update_tray();
            }
            WorkerEvent::FirmwareResult(message) => {
                self.status_override = Some(message);
                self.update_tray();
            }
        }
    }

    fn request_refresh(&self) {
        let _ = self.worker_tx.send(WorkerCommand::Refresh);
    }

    fn update_tray(&mut self) {
        let Some(snapshot) = self.latest_snapshot.as_ref() else {
            return;
        };

        let result = if let Some(tray) = self.tray.as_ref() {
            tray.update(
                snapshot,
                self.autostart_enabled,
                self.status_override.as_deref(),
            )
        } else {
            tray::Tray::new(
                snapshot,
                self.autostart_enabled,
                self.status_override.as_deref(),
            )
            .map(|tray| {
                self.tray = Some(tray);
            })
        };

        if let Err(error) = result {
            tracing::error!(%error, "failed to update tray");
        }
    }
}

fn spawn_worker(
    proxy: EventLoopProxy<UserEvent>,
    worker_rx: mpsc::Receiver<WorkerCommand>,
    config: AppConfig,
) {
    thread::spawn(move || {
        let mut selected_key = config.selected_device_key;
        let refresh_interval = Duration::from_secs(config.refresh_interval_seconds.max(1));
        let mut should_refresh = true;

        loop {
            if should_refresh {
                send_snapshot(&proxy, selected_key.as_deref());
                should_refresh = false;
            }

            match worker_rx.recv_timeout(refresh_interval) {
                Ok(WorkerCommand::Refresh) => {
                    should_refresh = true;
                }
                Ok(WorkerCommand::SelectDevice(key)) => {
                    selected_key = key;
                    should_refresh = true;
                }
                Ok(WorkerCommand::Firmware) => {
                    send_firmware(&proxy, selected_key.as_deref());
                }
                Ok(WorkerCommand::Stop) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    should_refresh = true;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });
}

fn send_snapshot(proxy: &EventLoopProxy<UserEvent>, selected_key: Option<&str>) {
    match MouseBattery::new() {
        Ok(mouse_battery) => {
            let snapshot = mouse_battery.snapshot(selected_key);
            let _ = proxy.send_event(UserEvent::Worker(WorkerEvent::Snapshot(snapshot)));
        }
        Err(error) => {
            tracing::error!(%error, "failed to initialize HID for refresh");
            let snapshot = BatterySnapshot {
                devices: Vec::new(),
                selected_key: None,
                status: crate::battery::BatteryStatus::NotFound,
            };
            let _ = proxy.send_event(UserEvent::Worker(WorkerEvent::Snapshot(snapshot)));
        }
    }
}

fn send_firmware(proxy: &EventLoopProxy<UserEvent>, selected_key: Option<&str>) {
    let message = match MouseBattery::new()
        .ok()
        .and_then(|mouse_battery| mouse_battery.get_firmware_version(selected_key))
    {
        Some((device, version)) => format!("{} firmware: {version}", device.name),
        None => "Unable to retrieve firmware version. Is the mouse connected?".to_string(),
    };

    let _ = proxy.send_event(UserEvent::Worker(WorkerEvent::FirmwareResult(message)));
}
