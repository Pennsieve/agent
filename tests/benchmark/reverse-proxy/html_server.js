#!/usr/bin/env node

const http = require('http');

const port = 40404;

const server = http.createServer((req, res) => {
  res.writeHead(200, { 'Content-Type': 'text/html' });
  let now = process.hrtime();
  let ts = now.toString();
  res.write("<!doctype html>\n");
  res.write("<html>\n");
  res.write("<body>\n");
  res.write(`<h1>${ts}</h1>\n`);
  res.write("</body>\n");
  res.write("</html>\n");
  res.end();
});

console.log(`Listening on port ${port}`);
server.listen(port);
