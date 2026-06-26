# MoniServ

This is a simple tool written in Rust, that helps monitor services and do simple recoveries.

It has built-in support for monitoring HTTP requests.  But you can always easily extend MoniServ's ability by providing scripts as custom commands.

In addition to logs, failures can be reported by emails.

## How to Build

This version is built by Rust 1.96.0.

If you haven't had Rust installed, consult the following page:
https://rust-lang.org/tools/install/

You may need to install additional packages.  Follow your system's installer's guides.

Just enter the directory and run:

`cargo build --release`

## How to Install

The build process creates a `target/release/monitor-services.exe` file.  Just copy it anywhere you want.

## Configuration

All configurations reside in a single directory as a set of YAML files.  These file's names must end with ".yml".

The YAML files can have two kinds of root level entries: monitor targets and the common settings.

All settings can be put in any .yml files in the directory.

The "_common" root level key starts a map giving common settings.  All other root level keys starts a map describing a monitor target.

Although you can distribute entries of "_common" in multiple files, and these settings will be merged, this is not recommended.

A monitor target describes what you want to monitor and how.

Make a copy of [samples/config](samples/config/), and try to edit.  The samples contain many comments to help you understand the entries.

## How to Run

Just execute the command:

`monitor-services config_dir`

Here, `config_dir` is the directory containing the .yml configuration files.