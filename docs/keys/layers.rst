Layer Keys
===============
The layers of a space cadet driver can be manipulated using
these keys:

+------------------------------------+----------------------------------------------------------------------+
| Key                                | Description                                                          |
+------------------------------------+----------------------------------------------------------------------+
| ``TG(layer)``                      | Toggle a layer.                                                      |
+------------------------------------+----------------------------------------------------------------------+
| ``MO(layer)``                      | Momentarily enable a layer until the key is released.                |
+------------------------------------+----------------------------------------------------------------------+
| ``AL(layer)``                      | Activate a layer.                                                    |
+------------------------------------+----------------------------------------------------------------------+
| ``LT(layer,key,hold_duration_ms)`` | Enable a layer when held; emit a key when tapped.                    |
+------------------------------------+----------------------------------------------------------------------+
| ``OSL(layer)``                     | Temporarily enable a layer until the next key is pressed + released. |
+------------------------------------+----------------------------------------------------------------------+

.. glossary::

    ``MACRO(...)``

        A macro key is a collection of keys that are pressed and released
        in order when the physical key is released.

    ``TG(LAYER)``

        Toggle whether a ``LAYER`` is turned on or off.

    ``MO(LAYER)``

        Enable a ``LAYER`` when the key is pressed, then disable the ``LAYER``
        when the key is released.

    ``AL(LAYER)``:

        Activate a ``LAYER``.

    ``OSL(LAYER)``:

        Enable a ``LAYER`` when pressed. The layer is disabled after another key
        is pressed and released. QMK calls this a "one-shot-layer" - it
        allows you to perform temporary layer switching without having
        to hold down a key.

    ``LT(LAYER,KEY,HOLD_DURATION_MS)``:

        Emit a ``KEY`` when tapped (i.e. pressed and released quickly).
        Enable a ``LAYER`` when held for more than ``HOLD_DURATION_MS``.
        The layer is disabled when the held key is released.