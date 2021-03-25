---
title: Getting started
keywords: pennsieve agent tutorial
last_updated: November 02, 2018
tags: [tutorial]
summary: Getting started with the Pennsieve Agent
sidebar: agent_sidebar
permalink: agent_tutorial.html
folder: agent
---

## Configuring the Pennsieve Agent
The first step is to configure the agent to use your Pennsieve account by registering an API key/secret. Use the Pennsieve web application to [generate API credentials](http://help.pennsieve.com/developers/configuration/creating-an-api-key-for-the-pennsieve-clients )

The agent configuration is built based on a config file stored in ~/.pennsieve/config.ini.

Navigate to the folder where the Pennsieve Agent is installed. Look [here](https://developer.pennsieve.io/agent/index.html) for the default locations for each operating system. Now, use the ```config wizard``` command to create a new config file. Provide a name for your new profile and the API key and secret that you created in the web application.

```bash
$ pennsieve config wizard
```

New profiles can be added to your configuration file using the ```profile create``` command.

```bash
$ pennsieve profile create
Create a new profile:
  Profile name: [default]  myProfile
  API token: xxxx
  API secret: xxxx
```

Verify that the newly created profile is selected using:

```bash
$ pennsieve profile
Current profile: <new Profile>
```

If the newly selected profile is not selected as the active profile, use the ```profile switch``` command to switch the active profile to the newly created profile. To verify you can access your account, use the ```whoami``` command to request some information about your Pennsieve account.

```bash
$ pennsieve whoami
+-----------------+-----------------------------------------------------+
| NAME            | user@email.com                                      |
| USER ID         | N:user:xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx         |
| ORGANIZATION    | User Oganization                                    |
| ORGANIZATION ID | N:organization:xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx |
+-----------------+-----------------------------------------------------+
```

### Using environment variable overrides
Pennsieve credentials can also be configured using the following environment variables.

```
PENNSIEVE_API_TOKEN
PENNSIEVE_API_SECRET
```

As long as both of these environment variables are set, the agent will use these credentials instead of any profiles found in the config.ini file.

## Getting help
You can find documentation for the agent using the ```--help``` option.
For example:

```bash
$ pennsieve --help
```
or
```bash
$ pennsieve upload-status --help
```
