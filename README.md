# power-supply-ieee488-gpib-controller

Control Agilent 6621A power supply over GPIB interface


## Serial adapter

Forked snapshot of [AR488](https://github.com/andreyk0/AR488) GPIB IEEE-488 serial adapter.

Connector: [FTDI USB serial adapter](https://microcontrollerslab.com/ftdi-usb-to-serial-converter-cable-use-linux-windows/).

Example `picocom` session

``` bash
picocom --baud 115200 --imap lfcrlf --echo /dev/ttyUSB0
```

Default address / print command results

```
++addr 5
++auto 1
```

Send some commands

```
id?
HP6621A

sts? 1
  1

err?
  6

test?
  0

vset 1 1.2; vset 2 2.3
```

