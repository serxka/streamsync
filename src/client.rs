use actix::prelude::*;
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};

use std::time::{Duration, Instant};

use crate::server::{ClientAction, ClientData, ServerCommand, VideoServer};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(40);

pub struct VideoClient {
	heartbeat: Instant,
	uid: u16,
	s_addr: Addr<VideoServer>,
}

impl VideoClient {
	pub fn new(s_addr: Addr<VideoServer>) -> Self {
		Self {
			heartbeat: Instant::now(),
			uid: 0,
			s_addr,
		}
	}

	fn heartbeat(&self, ctx: &mut <VideoClient as Actor>::Context) {
		let uid = self.uid;
		ctx.run_interval(HEARTBEAT_INTERVAL, move |actor, ctx| {
			if Instant::now().duration_since(actor.heartbeat) > CLIENT_TIMEOUT {
				println!("Hearbeat failed closing connection (uid{})", uid);
				ctx.stop();
			} else {
				ctx.ping(b"");
			}
		});
	}
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for VideoClient {
	fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
		match msg {
			Ok(ws::Message::Ping(msg)) => {
				self.heartbeat = Instant::now();
				ctx.pong(&msg);
			}
			Ok(ws::Message::Pong(_)) => {
				self.heartbeat = Instant::now();
			}
			Ok(ws::Message::Text(msg)) => {
				let data: IncomingRequest = match serde_json::from_str(&msg) {
					Ok(d) => d,
					Err(_) => {
						ctx.text("{\"error\":\"invalid structure\"}");
						return;
					}
				};
				match data {
					IncomingRequest::SendState(mut state) => {
						state.uid = self.uid;
						self.s_addr.do_send(ClientAction::UpdateState(state));
					}
					IncomingRequest::GetState => {
						self.s_addr
							.send(ClientAction::GetState)
							.into_actor(self)
							.then(|res, act, ctx| {
								match res {
									Ok(res) => ctx.text(
										serde_json::to_string(&res.get_state().as_command(act.uid))
											.unwrap(),
									),
									_ => ctx.stop(),
								}
								fut::ready(())
							})
							.wait(ctx);
					}
				}
			}
			Ok(ws::Message::Close(reason)) => {
				println!("Client uid{} left session, reason: {:?}", self.uid, reason);
				ctx.close(reason);
				ctx.stop();
			}
			Err(err) => {
				println!(
					"Client uid{} websocket error -- dropping connection, error: {:?}",
					self.uid, err
				);
				ctx.stop();
			}
			_ => (),
		}
	}
}

impl Actor for VideoClient {
	type Context = ws::WebsocketContext<Self>;

	fn started(&mut self, ctx: &mut Self::Context) {
		// GET THE BLOOD PUMPING
		self.heartbeat(ctx);

		// Get the actix address of the client actor and send it to the server
		// Then map the result of that into our ID
		let addr = ctx.address();
		self.s_addr
			.send(ClientAction::Connect(addr.recipient()))
			.into_actor(self)
			.then(|res, act, ctx| {
				match res {
					Ok(res) => act.uid = res.get_uid(),
					_ => ctx.stop(),
				}
				fut::ready(())
			})
			.wait(ctx);
	}
	fn stopping(&mut self, _: &mut Self::Context) -> Running {
		// Send the message that we have disconnect from the server
		self.s_addr.do_send(ClientAction::Disconnect(self.uid));
		Running::Stop
	}
}

#[derive(Deserialize, Debug, Serialize)]
enum IncomingRequest {
	SendState(ClientData),
	GetState,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct OutgoingCommand(pub ServerCommand);

impl Handler<OutgoingCommand> for VideoClient {
	type Result = ();
	fn handle(&mut self, msg: OutgoingCommand, ctx: &mut Self::Context) {
		ctx.text(serde_json::to_string(&msg.0).unwrap());
	}
}
