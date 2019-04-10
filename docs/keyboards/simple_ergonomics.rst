Simple Ergonomics
==================

This is an example keyboard that performs simple key remapping to get
slightly better ergonomics. The ``CAPSLOCK`` key doesn't deserve to
be 1 key away from the home row...

+-----------------+-----------------+
| Before          | After           |
+=================+=================+
| ``KC_ESC``      | ``KC_CAPSLOCK`` |
+-----------------+-----------------+
| ``KC_TAB``      | ``KC_ESC``      |
+-----------------+-----------------+
| ``KC_CAPSLOCK`` | ``KC_TAB``      |
+-----------------+-----------------+

In addition to simple key remapping, the left and right shift keys
are mapped to mapped to space cadet keys. The behavior of space cadet
keys depends on whether the key is held or tapped:

+----------------+----------------+--------+
|                | New Behavior            |
+================+================+========+
| Old Behavior   | Held           | Tapped |
+----------------+----------------+--------+
| ``LEFTSHIFT``  | ``LEFTSHIFT``  | ``(``  |
+----------------+----------------+--------+
| ``RIGHTSHIFT`` | ``RIGHTSHIFT`` | ``)``  |
+----------------+----------------+--------+


Layers
--------------
.. literalinclude:: ../../keyboards/simple_ergonomics/layers.json
    :language: json

Matrix
--------------
.. literalinclude:: ../../keyboards/simple_ergonomics/matrix.json
    :language: json