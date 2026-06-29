# MoniServ

Table of Contents
- [How to Build](#how-to-build)
- [How to Install](#how-to-install)
- [How to Run](#how-to-run)
- [Configuration](#configuration)
- [- Settings Under the "_common" Key](#settings-under-the-_common-key)
- [- Settings Under A Target](#settings-under-a-target)

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

## How to Run

Just execute the command:

`monitor-services config_dir`

Here, `config_dir` is the directory containing the .yml configuration files.

## Configuration

All configurations reside in a single directory as a set of YAML files.  These file's names must end with ".yml".

The YAML files can have two kinds of root level entries: monitor targets and the common settings.

All settings can be put in any .yml files in the directory.

The "_common" root level key starts a map giving common settings.  All other root level keys starts a map describing a monitor target.

Although you can distribute entries of "_common" in multiple files, and these settings will be merged, this is not recommended.

A monitor target describes what you want to monitor and how.

Make a copy of [samples/config](samples/config/), and try to edit.  The samples contain many comments to help you understand the entries.

### Settings Under the "_common" Key

| Key                      | Data Type  | Value Description                |
|:-------------------------|:-----------|:---------------------------------|
| `report_to`              | string     | The email address, to which an error report will be sent to.  If not specified, no reports will be sent. |
| `report_from`            | string     | When sending the email, use this value as the from address.  If not specified, no reports will be sent. |
| `smtp_host`              | string     | The SMTP host for sending email.  If not specified, no reports will be sent. |
| `smtp_port`              | integer    | The port number for submitting the email.  If not specified, no reports will be sent. |
| `smtp_user`              | string     | The user name that will be used to log into the SMTP server.  If not specified, no reports will be sent. |
| `smtp_pass`              | string     | The password that will be used to log into the SMTP server.  If not specified, no reports will be sent. |
| `smtp_starttls`          | boolean    | Make the email sender to start TLS to switch to a secure channel.  Default: false. |
| `smtp_no_verify_hostname`| boolean    | Skip host name verification. Default: false. |
| `smtp_no_check_certificate`| boolean  | Skip certification checking. Default: false. |
| `check_interval_secs`    | integer    | Number of seconds between to consecutive checks. Default: 30. |
| `min_log_level`          | string     | One of `trace`, `debug`, `info`, `warning`, `error`. |
| `connect_timeout`        | integer    | Number of seconds as connection timeout if applicable.  Default: 30. |
| `read_timeout`           | integer    | Number of seconds as read timeout if applicable.  Default: 30. |
| `timeout`                | integer    | Number of seconds as general timeout if applicable.  Default: 30. |

### Settings Under A Target

| Key                   | Data Type  | Value Description                |
|:----------------------|:-----------|:---------------------------------|
| `kind`                | string     | The kind of service to monitor.  One of `web`, `custom`. |
| `url`                 | string     | Url of the `web` service to monitor. |
| `method`              | string     | The method for accessing the service if applicable.  For `web`, it is one of `OPTIONS`, `GET`, `POST`, `PUT`, `DELETE`, `HEAD`, `TRACE`, `CONNECT`, `PATCH`. |
| `body`                | string     | Body of the request if applicable. |
| `check_interval_secs` | integer    | Number of seconds between to consecutive checks. Default: global setting. |
| `expect_status_code`  | integer    | The status code of the response to be matched. |
| `expect_match`        | boolean    | If supplied, it is the regular expression that the response body must match against.  In this case, the response body must be texts.  Default: None. |
| `expect_unmatch`      | integer    | If supplied, it is the regular expression that the response body must NOT match against.  In this case, the response body must be texts.  Default: None. |
| `custom_check_cmd`    | string     | The custom command to execute when `kind`'s value is `custom`.  The exit status code must be 0 to be treated as success. |
| `recovery_cmd`        | integer    | The command to execute if check failed. |
| `connect_timeout`     | integer    | Number of seconds as connection timeout if applicable.  Default: global setting. |
| `timeout`             | integer    | Number of seconds as general timeout if applicable.  Default: global setting. |
| `read_timeout`        | integer    | Number of seconds as read timeout if applicable.  Default: global setting. |
