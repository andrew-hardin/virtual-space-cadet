How it works
===============
The space cadet driver presents itself as a virtual device by
intercepting and interpreting events from a physical device.
The purpose of this document is to flesh out what that means.

Grab an input device
------------------------
The space cadet driver starts by "grabbing" an input device,
e.g. your keyboard device at ``/dev/input/event4``. This
is an exclusive grab - any events sent by the keyboard will
be _intercepted_ by the driver.

State matrix
----------------
The driver polls the input device at a fixed frequency to
check if any new events have occurred. Upon receiving an event
(e.g. "``KC_A`` was pressed"), the driver maps that event to a
position in a 2D matrix. Like a physical keyboard's underlying
firmware, this matrix records the binary state of every key 
(``{key_up = 0, key_down = 1}``).

The driver watches for changes in the state matrix. Key presses
are detected when a state changes from `0 -> 1`. Releases are
detected a state changes from `1 -> 0`. When a key press or
release is detected, the _position_ of the change is
used to determine which action to take (e.g. send `KC_A`).
The mapping from state change to action is handled by
a collection of layers.

Layers
----------
The virtual keyboard's layout is composed of a series of layers.
Each layer is a 2D matrix of key codes, and its dimension matches
the driver's state matrix. Layers are stacked on top of
one another - with higher layers taking precedence.

When the driver detects a change in the state matrix
(e.g. ``press @ {row 0, col 0}``), it loops through every
enabled layer and forwards the event to the first non-transparent
key it finds.

For example, the keyboard driver below has three 1x1 layers.
When a ``key_press`` event is detected at ``{row 0, col 0}``, the
driver starts at the highest layer and goes down until it finds a
non-transparent key - in this case ``KC_A``.

.. code-block:: text

    (disabled) layer 1: [KC_B]         // candidate #1 skipped because layer disabled
    (enabled)  layer 2: [TRANSPARENT]  // candidate #2 skipped because transparent
    (enabled)  layer 0: [KC_A]         // candidate #3 accepted

For a better description of layers, please refer to QMK's
discussion of `layers <https://beta.docs.qmk.fm/detailed-guides/keymap>`_.
General concepts should transfer to this project. 

Key Codes
----------------
What happens after an event is passed to a key code depends
entirely on what type of key it is.  In the simplest case,
when a regular key code receives an event it immediately writes
an event to the output device.

In addition to traditional keyboard codes, advanced
codes such as macros, modifiers, and speed/tap sensitive keys
can be included in a layer. The :doc:`key index<keys/index>`
describes all possible keys.

