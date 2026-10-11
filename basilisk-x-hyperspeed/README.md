# Razer Basilisk X HyperSpeed

Drives the Razer Basilisk X HyperSpeed from InputHub, and is the example of
a plugin that brings a device.

| What | How |
| --- | --- |
| Buttons | taken by InputHub through the kernel, like any mouse's |
| Pointer speed, 100 to 16000 DPI | the plugin's code, in Razer's protocol |
| The speeds the DPI button goes through, 1 to 5 | the same |
| Report rate, 125, 500 or 1000 Hz | the same |
| Battery | the same |

## What is in it

```
manifest.toml                             what the plugin is; asks for `device`
devices/basilisk-x-hyperspeed/device.toml the mouse: its name, its USB ids, its settings
plugin.wasm                               the code, built from `code/`
code/                                     the source of the code
```

`device.toml` lists the settings as data: InputHub shows them without the
plugin drawing anything. The code answers three questions about the one
device `usb` names, and is handed nothing else:

| InputHub calls | The code gives |
| --- | --- |
| `get(setting)` | the value of the setting at that place in `device.toml`, or less than 0 if the device has not got it just now |
| `set(setting, value)` | 0 when the device took it |
| `battery()` | percent, plus 256 while charging, or less than 0 |

| The code can call | Which |
| --- | --- |
| `send(report)` | sends the device a feature report |
| `read(report)` | reads one |
| `wait(ms)` | waits for the device, three seconds in all for a run |
| `log(line)` | writes a line in InputHub's log |

## Access to the mouse

A plugin cannot give itself access to a device. InputHub's udev rule,
`packaging/linux/70-inputhub.rules`, has a line for Razer; without it the
buttons work and the settings and battery do not.

## Building the code

```
rustup target add wasm32-unknown-unknown
cd code
cargo build --release
cp ../../.target/wasm32-unknown-unknown/release/basilisk_x_hyperspeed.wasm ../plugin.wasm
```

## Trying it

```
inputhub-ctl plugin install /path/to/basilisk-x-hyperspeed
inputhub-ctl settings razer
```
