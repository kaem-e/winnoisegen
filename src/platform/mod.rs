#[cfg(windows)]
mod windows;

#[cfg(windows)]
use windows::{EventLoopProxyWindows, EventLoopWindows};

#[cfg(windows)]
type EventLoopType = EventLoopWindows;

#[cfg(windows)]
type EventLoopProxyType = EventLoopProxyWindows;

#[cfg(not(target_os = "windows"))]
compile_error!("This application only supports Windows.");

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
/// Named Enumeration for application events
pub enum AppEvent {
	PlaybackToggle,
	AudioDeviceSwitched,
	SystemThemeChanged,
	QuitApplication,
}

/// Named Enumeration for theme variants
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[repr(u8)]
pub enum Theme {
	Light,
	Dark,
}

/// This trait need to be implemented for the platform-specific EventProxy types
trait EventLoopImpl {
	fn new() -> EventLoopType;
	fn create_event_proxy(&self) -> EventLoopProxyType;
	fn pump(&self) -> anyhow::Result<AppEvent>;
}

/// This trait need to be implemented for the platform-specific EventProxy types
trait EventLoopProxyImpl: Clone + std::fmt::Debug {
	fn send_event(&self, event: AppEvent) -> anyhow::Result<()>;
}

// ----------- Event Loop -----------

pub struct EventLoop(EventLoopType);

impl EventLoop {
	pub fn new() -> Self {
		let event_loop = EventLoopType::new();

		Self(event_loop)
	}

	pub fn create_event_proxy(&self) -> EventLoopProxy {
		EventLoopProxy(self.0.create_event_proxy())
	}

	pub fn pump(&self) -> anyhow::Result<AppEvent> {
		self.0.pump()
	}
}

// ------------- Proxy --------------

#[derive(Clone, Debug)]
pub struct EventLoopProxy(EventLoopProxyType);

impl EventLoopProxy {
	pub fn send_event(&self, event: AppEvent) -> anyhow::Result<()> {
		self.0.send_event(event)
	}
}

#[cfg(windows)]
pub use windows::get_current_theme;
