# IPWatch
`ipwatch` is a tool for monitoring a `netlink` socket for changes in IP address which in turn restarts services that would otherwise fail to recognize the change.

## Options
```
ipwatch

USAGE:
    ipwatch [OPTIONS] <command>...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -i, --interface <IFNAME>    Interface to monitor using regexp (default .*)
    -t, --timeout <timeout>     Specify the grace period between SIGTERM and SIGKILL (default 10)

ARGS:
    <command>...    Command to run
```

## Example
To start [openpyn](https://github.com/jotyGill/openpyn-nordvpn) so that change on `wlan0` will cause it to restart, you may run:
`ipwatch -i wlan0 openpyn us`.

## Install
Using cargo, the installation is as simple as:
`cargo install ipwatch`
