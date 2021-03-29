---
title: "Installation of the Pennsieve Agent"
keywords: sample homepage
sidebar: agent_sidebar
permalink: index.html
summary: Installing the Pennsieve Agent
---

The Pennsieve Agent is an application that runs natively on Mac, Linux and Windows and is required for optimal performance of the MATLAB and Python clients. In addition, the Pennsieve Agent provides a Command Line Interface (CLI) for the Pennsieve platform.


## Installing the Pennsieve Agent

### Windows

Download and install [`Pennsieve-{{ site.data.agent_version }}-x86_64.msi`](https://github.com/Pennsieve/agent/releases/download/{{ site.data.agent_version }}/Pennsieve-{{ site.data.agent_version }}-x86_64.msi).

- The Pennsieve home directory will be created in `C:\Users\<user>\.pennsieve`.
- The agent executable will be installed to `C:\Program Files\Pennsieve\pennsieve.exe`.
- The agent will automatically be added to your path.

### macOS

Download and install [`pennsieve-{{ site.data.agent_version }}.pkg`](https://github.com/Pennsieve/agent/releases/download/{{ site.data.agent_version }}/pennsieve-{{ site.data.agent_version }}.pkg).

- The Pennsieve home directory will be created in `/Users/<user>/.pennsieve`.
- The agent executable will be installed to `/usr/local/opt/pennsieve/bin/pennsieve`.
- Run `echo 'export PATH="/usr/local/opt/pennsieve/bin:$PATH"' >> ~/.profile && source ~/.profile` to add the agent to your path.


### Debian Linux

Download and install [`pennsieve_{{ site.data.agent_version }}_amd64.deb`](https://github.com/Pennsieve/agent/releases/download/{{ site.data.agent_version }}/pennsieve_{{ site.data.agent_version }}_amd64.deb).

- The Pennsieve home directory will be created in `/home/<user>/.pennsieve`.
- The agent executable will be installed to `/opt/pennsieve/bin/pennsieve`.
- Run `echo 'export PATH="/opt/pennsieve/bin:$PATH"' >> ~/.profile && source ~/.profile` to add the agent to your path.

## Running the Pennsieve Agent

To run the Pennsieve Agent, or use the CLI tools, open a terminal and run

```bash
$ pennsieve version
```

If this does not work, see the instructions above for adding the agent executable to your path.
