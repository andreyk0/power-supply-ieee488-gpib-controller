# power-supply-ieee488-gpib-controller

Control Agilent 6621A power supply over GPIB interface

## NEXT

Info view

* button short press - both channels on/off
* button long press - file selector UI
* rotary encoder - V adjust
* rotary encoder (while pressed) - I adjust
* rotary encoder (press/release, no turn) - [ch1, ch2, ch1+2]

File view

* encoder scroll, press to run
* button cancel, back to info screen



## Serial adapter

Forked snapshot of [AR488](https://github.com/andreyk0/AR488) GPIB IEEE-488 serial adapter.

Connector: [FTDI USB serial adapter](https://microcontrollerslab.com/ftdi-usb-to-serial-converter-cable-use-linux-windows/).

Example `picocom` session

```bash
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

Interface: 4wire (Gnd, Vcc, Rx, Tx), 115200, no flow control.

Signal levels: TTL 3.3V (works with 5V FTDI cable too).


### MCU serial

Testing MCU serial - to - UART via USB serial

``` bash
picocom --baud 115200 --imap lfcrlf --echo /dev/ttyUSB0
picocom --baud 115200 --imap crcrlf --echo /dev/ttyACM0
```
