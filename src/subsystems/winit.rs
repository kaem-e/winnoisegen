use log::{error, info};
use winit::{
	application::ApplicationHandler, event::WindowEvent, event_loop::ActiveEventLoop,
	window::WindowId,
};

use crate::app::{App, UserEvent};

impl ApplicationHandler<UserEvent> for App {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		if let Err(e) = self.initialize(event_loop) {
			error!("App Initialization Error: {:#}", e);
			event_loop.exit();
		};
	}

	fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
		if let Err(e) = self.handle_event(event_loop, event) {
			error!("User Event Handling Error: {:#}", e);
		};
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
		if let Err(e) = self.handle_event(event_loop, UserEvent::WinitWindowEvent(event, id)) {
			error!("Window Event Handling Error: {:#}", e);
		};
	}

	fn exiting(&mut self, _: &ActiveEventLoop) {
		info!("event loop exit requestied");
	}
}
