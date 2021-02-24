const video = document.getElementById("video");

let wsConn = null;
let playedFirstTime = false;

const wsEvents = {
	open: function() {
		console.log("Connected to WebSocket!");
	},
	message: function(event) {
		console.log('incumming data: ', event.data);

		// Will only ever get called once
		if (playedFirstTime === false) {
			updateVideoState(JSON.parse(event.data));
			playedFirstTime = true;
			return;
		}

		disableEvents();
		
		updateVideoState(JSON.parse(event.data));
		
		enableEvents();
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

function sendSHIT() {
	wsConn.send(`{"SendState":{"time":${video.currentTime},"playing":${!video.paused},"uid":0}}`);
}

function updateVideoState(data) {
	video.currentTime = data.time;
	if (data.playing === true)
		video.play();
	else
		video.pause();
}

const videoEvents = {
	seeked: function() {
		sendSHIT();
	},
	play: function() {
		if (playedFirstTime === false) 
			wsConn.send('"GetState"');
		else
			sendSHIT();
	},
	pause: function() {
		sendSHIT();
	}
};

function disableEvents() {
	video.onseeked = null;
	video.onplay = null;
	video.onpause = null;
}

function enableEvents() {
	video.onseeked = videoEvents.seeked;
	video.onplay = videoEvents.play;
	video.onpause = videoEvents.pause;
}

window.onload = function() {
	enableEvents();
	wsConnect();
};
