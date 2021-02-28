# power-supply-ieee488-gpib-controller

Control Agilent 6621A power supply over GPIB interface

## Interface

Info view

* button short press - both channels on/off
* button long press - file selector UI
* rotary encoder - adjust by 0.1 (V/I)
* rotary encoder (while pressed) - adjust by 1 (V/I)
* rotary encoder short press - flip between I/V adjustment
* rotary encoder long press - select Ch1, Ch2, Both

File view

* encoder scroll, press to run
* button cancel, back to info screen

## SDCard

* `BOOT` file (root directory) is loaded on startup
* root directory is listed in the file selector screen (64 entries max, each 32 char max)

Example [boot file](etc/BOOT).


## Display

Repurposed [Reprap controller board](https://reprap.org/wiki/RepRapDiscount_Full_Graphic_Smart_Controller).


## Pin assignments

See [STM32 Cube project file](https://www.st.com/en/ecosystems/stm32cube.html) and/or [init code](src/bin/controller.rs) / [types.rs](src/types.rs).


## GPIB Serial adapter

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

## Openocd standalone

Use `openocd` without GDB.

LTO (link time optimizaions) breaks GDB because linker generates an ELF that still refers to (now missing) inlined sections. OTOH without LTO binary doesn't fit into the flash.

Connect to the running `openocd`:

``` bash
nc localhost 4444
```

Use built-in `program` script to erase/load new binary:

```
program target/thumbv7m-none-eabi/debug/controller
```

Enable debug output from the chip:

```
arm semihosting enable
```

Reset chip

```
reset
```
