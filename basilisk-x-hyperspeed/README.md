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
| `get(setting)` | the value of the setting at that place in `device.toml`, or nothing if the device has not got it just now |
| `set(setting, value)` | whether the device took it |
| `battery()` | how full it is and whether it is charging, or nothing |

| The code can call | Which |
| --- | --- |
| `device::send(report)` | sends the device a feature report |
| `device::read(report)` | reads one |
| `wait(ms)` | waits for the device, three seconds in all for a run |
| `log(line)` | writes a line in InputHub's log |

Both tables are the `inputhub-plugin` crate, from `sdk/inputhub-plugin` in
InputHub's repository: `device! { get: get, set: set, battery: battery }`
names the three functions to InputHub, and the rest are its functions. The
crate holds the little `unsafe` that crossing to InputHub and back takes,
so the code here has none and forbids it.

InputHub remembers what each setting was put at and puts it back when the
mouse wakes or a step comes back, so the code remembers nothing itself.

The source is one file for each thing:

```
code/src/lib.rs        which setting of device.toml is which
code/src/protocol.rs   a report to the mouse and its answer
code/src/speed.rs      the pointer's speed
code/src/steps.rs      the speeds the DPI button goes through
code/src/rate.rs       how often the mouse reports
code/src/power.rs      the battery
```

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
