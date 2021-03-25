---
title: Uploading files
keywords: pennsieve agent tutorial
last_updated: November 02, 2018
tags: [tutorial]
summary: Tutorial 2 - Uploading files to the Pennsieve platform
sidebar: agent_sidebar
permalink: agent_tutorial2.html
folder: agent
---

This tutorial describes how to upload files to the Pennsieve platform using the Pennsieve Agent. It assumes that the user has a Pennsieve account and has setup his/her password. It also assumes the user has configured the agent using the method described in [tutorial 1](/agent/agent_tutorial.html).

## Creating a dataset
{% include note.html content="If you already have created a dataset, you can skip this section." %}

All files on the Pennsieve platform belong to a dataset. Therefore, we first need to create a dataset before we can upload files to the platform. You create a dataset using the ```init``` command.

```bash
$ pennsieve init newDataset 'This is a test dataset for tutorial 2.'

Created dataset newDataset (N:dataset:xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx)
```

The dataset is now created. To verify, you can open the web application and see the dataset listed in your account. you can also list all datasets using the Agent with the ```datasets``` command.

```bash
$ pennsieve datasets

+-----------------------------+----------------------+
| DATASET                     | NAME                 |
+-----------------------------+----------------------+
| N:dataset:xxxxxxxx-xxxx-xx  | newDataset           |

```

## Uploading data
Now, let's upload some files to the dataset we just created. The `upload` command has multiple options to specify exactly which files should be uploaded. Some examples are provided below, but be sure to check out the [documentation](/agent/agent_upload.html) on this command for a complete view of its capabilities.

### Examples

#### Single files

Upload a single image file from the `~/Desktop/` folder to the `newDataset` dataset.

```bash
$ pennsieve upload --dataset newDataset ~/Desktop/image.jpg
```

#### Multiple files using shell expansion

Shell-based file expansion ("globbing") can be used to filter specific files to upload. For instance, we can specify that only DICOM files ending `.dcm` should be uploaded:

```bash
$ pennsieve upload --dataset newDataset ~/Desktop/*.dcm
```

Additionally, globbing can be used to specify multiple files across multiple directories to upload. 

```bash
$ pennsieve upload --dataset newDataset ~/Desktop/*.dcm  ~/My-Data/*.json
```

This will upload all `.dcm` files in `~/Desktop/` and all `.json` files in `~/My-Data/`.

When multiple paths are supplied, all entries after glob expansion must specify files and not directories, otherwise an error will be issued.

For instance,

```bash
$ pennsieve upload --dataset newDataset ~/Desktop/*.dcm  ~/My-Data/

upload error: When using multiple paths, all paths must be files. A directory was provided: ~/My-Data/
``

#### Uploading directory contents

Upload all files in the `~/Desktop/` folder to the `newDataset` dataset. This command will not recurse into subdirectories, it will only upload files that reside in `~/Desktop/`.

```bash
$ pennsieve upload --dataset newDataset ~/Desktop/
```

Upload all files in the `~/Desktop/` folder and all files in all subfolders of `~/Desktop/` using the `--recursive` flag. This flag will also preserve the folder structure in the Pennsieve platform.

```bash
$ pennsieve upload --dataset newDataset --recursive ~/Desktop/
```

Note: multiple directories cannot be specified when attempting to upload the contents of a directory.
