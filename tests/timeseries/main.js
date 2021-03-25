#!/usr/bin/env node
'use strict';

const DEBUG = Boolean(process.env.DEBUG)

const { createWriteStream } = require('fs');
const path = require('path');

const crc = require('crc-32');
const Log = require('log')
const log = new Log('debug', DEBUG ? undefined : createWriteStream('/dev/null'));

const timeseries = require('./timeseries_pb');

if (process.argv.length != 5) {
  console.error(`Usage: ${process.argv[1]} <port> <session-id> <request-json-file>`);
  process.exit(1);
}

const port = parseInt(process.argv[2], 10);
const session_id = process.argv[3];
const sample_file = process.argv[4];

log.debug(`- Reading ${sample_file}`);

const request = require(sample_file);
request['command'] = 'new';
request['session'] = session_id;

log.debug(request);

const WebSocket = require('ws');
const url = `ws://0.0.0.0:${port}/ts/query?session=${session_id}&package=${request.packageId}`;
const ws = new WebSocket(url);

let chunksReceived = null;
let stateCount = 0;
let canExit = false;

function sendNew() {
  log.debug('- Sent request');
  ws.send(JSON.stringify(request));
}

function sendNext() {
  ws.send('{"command": "next"}');
}

function sendClose() {
  ws.send('{"command": "close"}');
}

ws.binaryType = 'arraybuffer';

process.on('SIGINT', () => {
  sendClose();
})

process.on('SIGTERM', () => {
  sendClose();
})

ws.onopen = function open(evt) {
  sendNew();
};

ws.onmessage = function message(evt) {
  let payload = timeseries.AgentTimeSeriesResponse.deserializeBinary(evt.data);

  if (payload.hasState()) {
    const status = payload.getState().getStatus();
    const description = payload.getState().getDescription();

    if (status === 'ERROR') {
      console.error(`Error: ${description}`);
      ws.close();
      return;
    }

    stateCount++;
    if (canExit) {
      ws.close();
    } else if(stateCount >= 2) {
      sendClose();
      canExit = true;
    } else {
      sendNext();
    }
  } else {
    let data = payload.getChunk().getChannelsList()[0].getDataList();

    if (chunksReceived === null) {
      chunksReceived = 0;
    }

    chunksReceived++;

    for (let i in data) {
        log.debug(data[i].getTime() / 1000 + " " + data[i].getValue().toFixed(15));
    }

    //let buffer = Buffer.from(new Uint8Array(evt.data));
    //console.log(crc.buf(buffer));
    sendNext();
  }
};

ws.onerror = function error(evt) {
  if (evt.data) {
    console.error(`${evt.data}`);
  }
};

ws.onclose = function close(evt) {
  if (chunksReceived === null) {
    process.exit(1);
  } else {
    console.log(`received: ${chunksReceived}`);
    process.exit(0);
  }
};
