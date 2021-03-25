#!/usr/bin/env node

const http = require('http');

const port = 40404;

const server = http.createServer((req, res) => {
  res.writeHead(200, { 'Content-Type': 'application/json' });
  let now = process.hrtime();
  let ts = JSON.stringify({ sec: now[0], nsec: now[1] });
  res.end(ts);
});

console.log(`Listening on port ${port}`);
server.listen(port);
