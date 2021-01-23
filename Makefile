NAME?=blink
# debug | release
BUILD?=debug
ELF_TARGET:=target/thumbv7m-none-eabi/$(BUILD)/$(NAME)
BIN_TARGET:=target/$(NAME).bin

build: fmt
	cargo build $(if $(findstring release,$(BUILD)),--release,)

# Requires openocd running
debug: build
	arm-none-eabi-gdb -x openocd.gdb -q $(ELF_TARGET)

bin: build
	arm-none-eabi-objcopy -O binary $(ELF_TARGET) $(BIN_TARGET)

disassemble: build
	arm-none-eabi-objdump --disassemble $(ELF_TARGET) | less -S

doc:
	cargo doc --open

fmt:
	find src -type f -name '*.rs' | xargs rustfmt

flash: bin erase
	st-info --descr
	st-flash write $(BIN_TARGET) 0x8000000

erase:
	st-flash erase

clean:
	cargo clean

picocom:
	picocom -b 115200 --imap lfcrlf /dev/ttyACM0

.PHONY: \
	bin \
	build \
	clean \
	disassemble \
	erase \
	flash \
	picocom \
