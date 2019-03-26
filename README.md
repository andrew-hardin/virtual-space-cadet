Virtual Space-Cadet
===================

This is a virtual keyboard device that supports layers and advanced
key codes (inspired by QMK). The virtual keyboard device runs in user
space - there's no need to modify the kernel. It intercepts events
from an input device, interprets them, and emulates a new virtual
keyboard device:

**I need more pictures**

Why does this exist?
--------------------
The keyboard is my primary interface when working on a computer, but
traditional keyboard drivers are painfully simple. I was overjoyed to
discover the _flexibility_ that came with running QMK firmware on my
Kinesis Advantage.

Running QMK for 10 months taught me two obvious lessons:

1. My laptop's keyboard doesn't support layers.

2. Secure facilities don't like it when you bring a custom keyboard
   to work.<sup>1</sup>

A laptop keyboard should be a first-class input device, and advanced
capabilities should be available without needing to tweak the kernel.

Inspiration
-----------
This project showed me that it was possible to emulate a keyboard
layout in user space via `evdev` and `uinput`:

- [abrasive/spacefn-evdev](https://github.com/abrasive/spacefn-evdev)

The QMK project provided inspiration for layers and advanced key
codes:

- [qmk/qmk_firmware](https://github.com/qmk/qmk_firmware)


<sup>1</sup>
We trust keyboards running black-box firmware that was flashed onto
the device in China, but only if the keyboard has
`$large_manufacturer` printed on the box.
