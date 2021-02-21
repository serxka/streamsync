const video = document.getElementById("video");

let wsConn = null;
let eventActive = true;
let lastSenderID = null;
let lastTime = 0;
let myID = null;

const wsEvents = {
	open: function() {
		console.log("Connected to WebSocket!");
		wsConn.send("F:_");
	},
	message: function(event) {
		console.log(event.data);
		eventActive = false;
		const items = event.data.split(':', 3);
		switch (items[0]) {
			case '0': {
				video.currentTime = items[1];
				lastTime = items[1];
				lastSenderID = items[2];
				break;
			}
			case '1': {
				video.play();
				lastSenderID = items[2];
				break;
			}
			case '2': {
				video.pause();
				lastSenderID = items[2];
				break;
			}
			case 'F': {
				myID = items[2];
				break;
			}
			default: {
				console.error("unknown command: " + event.data);
				break;
			}
		}
		eventActive = true;
	},
	error: function(event) {
		console.error("WebSocket error: ", event);
	},
	close: function() {
		console.log("Closing WebSocket connection");
		wsConn = null;
	}
};

function wsConnect() {
	wsDisconnect(); // Disconnect in case we already are connected

	const wsUri = 
		(window.location.protocol === 'https:' ? 'wss://' : 'ws://')
		+ window.location.host
		+ '/ws';

	wsConn = new WebSocket(wsUri);
	console.log('Connecting to WebSocket at ', wsUri);

	wsConn.onopen = wsEvents.open;
	wsConn.onmessage = wsEvents.message;
	wsConn.onerror = wsEvents.error;
	wsConn.onclose = wsEvents.close;
}

function wsDisconnect() {
	if (wsConn !== null) {
		wsConn.close();
	}
}

const videoEvents = {
	seeked: function() {
		if (!eventActive) return;
		if (lastTime == video.currentTime) return;
		wsConn.send("0:" + video.currentTime);
	},
	play: function() {
		if (!eventActive) return;
		if (lastTime == video.currentTime) return;
		wsConn.send("1:_");
	},
	pause: function() {
		if (!eventActive) return;
		if (lastTime == video.currentTime) return;
		wsConn.send("2:_");
	}
};

window.onload = function() {
	video.addEventListener("seeked", videoEvents.seeked);
	video.addEventListener("play", videoEvents.play);
	video.addEventListener("pause", videoEvents.pause);

	wsConnect();
};
