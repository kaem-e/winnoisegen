/// Responsible for watching the shaders directory for changes, and when something in that directory changes just sending the event for it.
use log::*;
use notify::{Config, RecommendedWatcher, Watcher};
use winit::event_loop::EventLoopProxy;

use crate::{app::UserEvent, utils::log_err as _};

pub struct WatcherSubsystem(pub RecommendedWatcher);

impl WatcherSubsystem {
	pub fn new(proxy: EventLoopProxy<UserEvent>) -> anyhow::Result<Self> {
		let watcher = RecommendedWatcher::new(
			move |res| match res {
				Ok(e) => {
					proxy.send_event(UserEvent::ShaderFileChanged(e)).log_err();
				},
				Err(e) => error!("{:?}", e),
			},
			Config::default(),
		)?;

		Ok(Self(watcher))
	}
}
