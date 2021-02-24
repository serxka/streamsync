use actix::prelude::*;
use actix_files as fs;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;

mod client;
mod server;

use crate::client::VideoClient;
use crate::server::VideoServer;

async fn ws_connect(
	req: HttpRequest,
	stream: web::Payload,
	server: web::Data<Addr<VideoServer>>,
) -> Result<HttpResponse, Error> {
	ws::start(VideoClient::new(server.get_ref().clone()), &req, stream)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	let server = VideoServer::new().start();

	HttpServer::new(move || {
		App::new()
			.data(server.clone())
			.service(web::resource("/ws").to(ws_connect))
			.service(fs::Files::new("/static/", "static/"))
	})
	.bind("localhost:8080")?
	.run()
	.await
}
