Advanced Keys
===============

This is a catch-all category for keys that aren't simple
and have nothing to do with layers.


+------------------------------------+----------------------------------------------------------------------+
| Key                                | Description                                                          |
+------------------------------------+----------------------------------------------------------------------+
| ``MACRO(key,...)``                 | Execute a macro of keys.                                             |
+------------------------------------+----------------------------------------------------------------------+
| ``WRAP(key_outer,key_inner)``      | Wrap a key with another key, e.g. ``WRAP(KC_LEFTSHIFT,KC_9)``.       |
+------------------------------------+----------------------------------------------------------------------+
| ``SPACECADET(key_tap,key_held)``   | Emit different keys depending on whether the key is tapped or held.  |
+------------------------------------+----------------------------------------------------------------------+


.. glossary::

    ``MACRO(...)``

        A macro key is a collection of keys that are pressed and released
        in order when the physical key is released.

    ``WRAP(OUTER,INNER)``

        Wrap an ``INNER`` key with an ``OUTER`` key. Useful for getting
        shifted characters such as ``(){}``.

        :Example: ``WRAP(KC_LEFTSHIFT,KC_1)`` -> ``!``

    ``SPACECADET(KEY,HELD)``

        Emit a ``KEY`` when tapped, or act like ``HELD`` when held. This is
        similar to a one-shot-layer in the sense that key behavior depends
        on timing.

        The specific use case for this key is modifying shifts to emit
        parentheses when tapped. This would be accomplished via:

        :Example: ``SPACECADET(WRAP(KC_LEFTSHIFT,KC_9),KC_LEFTSHIFT)``
