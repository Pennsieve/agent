#!/usr/bin/env node
'use strict';

const WebSocket = require('ws');

const statusPort = process.argv[2] || 11235;
const ws = new WebSocket(`ws://0.0.0.0:${statusPort}`);

ws.onopen = function open(evt) {
  console.log("listen:connected");
};

ws.onmessage = function message(evt) {
  console.log(`message: ${evt.data}`);
};
