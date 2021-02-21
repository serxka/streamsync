use actix::prelude::*;
use actix_files as fs;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;

use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(40);

async fn ws_begin(
	req: HttpRequest,
	stream: web::Payload,
	server: web::Data<Addr<VideoControlServer>>,
) -> Result<HttpResponse, Error> {
	ws::start(VideoWebSocket::new(server.get_ref().clone()), &req, stream)
}

struct VideoWebSocket {
	heartbeat: Instant,
	id: usize,
	addr: Addr<VideoControlServer>,
}

impl VideoWebSocket {
	fn new(addr: Addr<VideoControlServer>) -> Self {
		Self {
			heartbeat: Instant::now(),
			id: 0,
			addr,
		}
	}

	fn heartbeat(&self, ctx: &mut <VideoWebSocket as Actor>::Context) {
		ctx.run_interval(HEARTBEAT_INTERVAL, |actor, ctx| {
			if Instant::now().duration_since(actor.heartbeat) > CLIENT_TIMEOUT {
				println!("WebSocket heartbeat failed, closing connection");
				ctx.stop();
			} else {
				ctx.ping(b"");
			}
		});
	}
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for VideoWebSocket {
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
				self.addr.do_send(ClientCommand {
					id: self.id,
					comm: Command::parse(&msg),
				});
			}
			Ok(ws::Message::Close(reason)) => {
				println!("WS Closing Connection, Reason: {:?}", reason);
				ctx.close(reason);
				ctx.stop();
			}
			Err(e) => {
				println!("WS Error: {:?}\nDropping connection.", e);
				ctx.stop();
			}
			_ => (),
		}
	}
}

impl Actor for VideoWebSocket {
	type Context = ws::WebsocketContext<Self>;

	fn started(&mut self, ctx: &mut Self::Context) {
		self.heartbeat(ctx);

		let addr = ctx.address();
		self.addr
			.send(Connect {
				addr: addr.recipient(),
			})
			.into_actor(self)
			.then(|res, act, ctx| {
				match res {
					Ok(res) => act.id = res,
					_ => ctx.stop(),
				}
				fut::ready(())
			})
			.wait(ctx);
	}
	fn stopping(&mut self, _: &mut Self::Context) -> Running {
		self.addr.do_send(Disconnect { id: self.id });
		Running::Stop
	}
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct OutgoingMessage(ClientCommand);

impl Handler<OutgoingMessage> for VideoWebSocket {
	type Result = ();
	fn handle(&mut self, msg: OutgoingMessage, ctx: &mut Self::Context) {
		ctx.text(msg.0.to_string());
	}
}

struct VideoControlServer {
	users: HashMap<usize, Recipient<OutgoingMessage>>,
}

impl Actor for VideoControlServer {
	type Context = Context<Self>;
}

impl VideoControlServer {
	pub fn new() -> Self {
		Self {
			users: HashMap::new(),
		}
	}

	fn send_message(&self, msg: ClientCommand) {
		for (id, addr) in self.users.iter() {
			if *id != msg.id {
				let _ = addr.do_send(OutgoingMessage(msg.clone()));
			}
		}
	}

	fn send_private_message(&self, msg: ClientCommand) {
		if let Some(addr) = self.users.get(&msg.id) {
			let _ = addr.do_send(OutgoingMessage(msg.clone()));
		}
	}
}

#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
	pub addr: Recipient<OutgoingMessage>,
}

impl Handler<Connect> for VideoControlServer {
	type Result = usize;
	fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> usize {
		let uid = rand::random();
		self.users.insert(uid, msg.addr);

		uid
	}
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
	pub id: usize,
}

impl Handler<Disconnect> for VideoControlServer {
	type Result = ();
	fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
		self.users.remove(&msg.id);
	}
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
struct ClientCommand {
	pub id: usize,
	pub comm: Command,
}

impl Handler<ClientCommand> for VideoControlServer {
	type Result = ();
	fn handle(&mut self, msg: ClientCommand, _: &mut Context<Self>) {
		match msg.comm {
			Command::GetID => self.send_private_message(msg),
			_ => self.send_message(msg),
		}
	}
}

impl std::fmt::Display for ClientCommand {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}", self.comm, self.id)
	}
}

#[derive(Clone)]
enum Command {
	SetTime(String), // SetTime (0)
	Play,            // Play (1)
	Pause,           // Pause (2)
	GetID,           // GetID (F)
	Unknown,
}

impl Command {
	fn parse(msg: &str) -> Command {
		// Check that we have at least three characters
		if msg.len() < 3 {
			return Command::Unknown;
		}
		// What a mess
		let mut iter = msg.chars();
		match iter.next().unwrap() {
			'0' => {
				// Check we have seperator
				if iter.next().unwrap() != ':' {
					return Command::Unknown;
				}

				// Collect time into it's own string
				let time = iter.collect::<String>();
				// Check that is valid decimal and return
				if time.parse::<f32>().is_err() {
					Command::Unknown
				} else {
					Command::SetTime(time)
				}
			}
			'1' => {
				// Check we have seperator and blank data
				match iter.cmp(":_".chars()) {
					Ordering::Equal => Command::Play,
					_ => Command::Unknown,
				}
			}
			'2' => {
				// Check we have seperator and blank data
				match iter.cmp(":_".chars()) {
					Ordering::Equal => Command::Pause,
					_ => Command::Unknown,
				}
			}
			'F' => {
				// Check we have seperator and blank data
				match iter.cmp(":_".chars()) {
					Ordering::Equal => Command::GetID,
					_ => Command::Unknown,
				}
			}
			_ => Command::Unknown,
		}
	}
}

impl std::fmt::Display for Command {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match &self {
			Command::SetTime(s) => write!(f, "0:{}", s),
			Command::Play => write!(f, "1:_"),
			Command::Pause => write!(f, "2:_"),
			Command::GetID => write!(f, "F:_"),
			Command::Unknown => write!(f, "?:_"),
		}
	}
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	let server = VideoControlServer::new().start();

	HttpServer::new(move || {
		App::new()
			.data(server.clone())
			.service(web::resource("/ws").to(ws_begin))
			.service(fs::Files::new("/static/", "static/"))
	})
	.bind("localhost:8080")?
	.run()
	.await
}
