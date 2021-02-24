use actix::prelude::*;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use crate::client::OutgoingCommand;

#[derive(Serialize, Copy, Clone)]
pub struct VideoState {
	pub time: f64,
	pub playing: bool,
}

impl VideoState {
	pub fn new() -> VideoState {
		VideoState {
			time: 0.0,
			playing: false,
		}
	}

	pub fn as_command(&self, uid: u16) -> ServerCommand {
		ServerCommand {
			time: self.time,
			source_uid: uid,
			playing: self.playing,
		}
	}
}

pub struct VideoServer {
	users: HashMap<u16, Recipient<OutgoingCommand>>,
	video_state: VideoState,
}

impl Actor for VideoServer {
	type Context = Context<Self>;
}

impl VideoServer {
	pub fn new() -> Self {
		Self {
			users: HashMap::new(),
			video_state: VideoState::new(),
		}
	}

	fn send_command(&self, msg: ServerCommand) {
		for (id, addr) in self.users.iter() {
			if *id != msg.source_uid {
				let _ = addr.do_send(OutgoingCommand(msg.clone()));
			}
		}
	}
}

#[derive(Copy, Clone)]
pub enum ClientActionResult {
	NewUid(u16),
	PlayState(VideoState),
	None,
}

impl ClientActionResult {
	pub fn get_uid(self) -> u16 {
		match self {
			ClientActionResult::NewUid(x) => x,
			_ => panic!("tried to unwrap enum to NewUid when it wasn't!"),
		}
	}
	pub fn get_state(self) -> VideoState {
		match self {
			ClientActionResult::PlayState(x) => x,
			_ => panic!("tried to unwrap enum to PlayState when it wasn't!"),
		}
	}
}

#[derive(Message)]
#[rtype(result = "ClientActionResult")]
pub enum ClientAction {
	Connect(Recipient<OutgoingCommand>),
	Disconnect(u16),
	UpdateState(ClientData),
	GetState,
}

impl Handler<ClientAction> for VideoServer {
	type Result = MessageResult<ClientAction>;

	fn handle(&mut self, msg: ClientAction, _: &mut Context<Self>) -> Self::Result {
		use ClientAction::*;
		match msg {
			Connect(addr) => {
				let mut uid;
				loop {
					uid = rand::random();
					if !self.users.contains_key(&uid) {
						break;
					}
				}
				self.users.insert(uid, addr);
				MessageResult(ClientActionResult::NewUid(uid))
			}
			Disconnect(uid) => {
				self.users.remove(&uid);
				MessageResult(ClientActionResult::None)
			}
			UpdateState(state) => {
				self.video_state.playing = state.playing;
				self.video_state.time = state.time;
				self.send_command(ServerCommand{time: state.time, source_uid: state.uid, playing: state.playing});
				MessageResult(ClientActionResult::None)
			}
			GetState => MessageResult(ClientActionResult::PlayState(self.video_state)),
		}
	}
}

#[derive(Deserialize, Debug, Serialize, Clone, Copy)]
pub struct ClientData {
	pub time: f64,
	pub uid: u16,
	pub playing: bool,
}

#[derive(Message, Serialize, Clone, Copy)]
#[rtype(result = "()")]
pub struct ServerCommand {
	pub time: f64,
	pub source_uid: u16,
	pub playing: bool,
}

impl Handler<ServerCommand> for VideoServer {
	type Result = ();
	fn handle(&mut self, msg: ServerCommand, _: &mut Context<Self>) {
		self.send_command(msg);
	}
}
