#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const WebSocket = require('ws');

const dataset = process.argv[2];
const targetDir = fs.realpathSync(process.argv[3]);

console.log(`Uploading ${targetDir}`);

const statusPort = process.argv[4] || 11235;
const ws = new WebSocket(`ws://0.0.0.0:${statusPort}`);

ws.onopen = function open(evt) {
  console.log("upload:connected");
  queueDirContents(function (files) {
    console.log('Queueing:');
    console.log(files);
    ws.send(JSON.stringify(files));
    process.exit(0);
  });
};

function queueDirContents(socketSend) {
  fs.readdir(targetDir, function (err, files) {
    if (err) {
      throw err;
    }
    const resolvedFiles = files.map(f => fs.realpathSync(path.join(targetDir, f)));
    const queueMessage = {
      message: "queue_upload",
      body: {
        dataset: dataset,
        files: resolvedFiles,
        recursive: false
      }
    };
    socketSend(queueMessage);
  });
}
