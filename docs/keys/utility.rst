Utility Keys
=============
Utility key codes are used to fill space:

+-----------------+-----------------------------+----------------------------------+
| Key             | Aliases                     | Description                      |
+-----------------+-----------------------------+----------------------------------+
| ``OPAQUE``      | ``X``, ``XX``, ``XXX``, ... | Ignore this key; a no-op.        |
+-----------------+-----------------------------+----------------------------------+
| ``TRANSPARENT`` | ``_``, ``__``, ``___``, ... | Check the key in the next layer. |
+-----------------+-----------------------------+----------------------------------+

.. glossary::

    ``OPAQUE``

        A black hole that swallows events but has no side effects.
        Useful when composing layers.

    ``TRANSPARENT``

        A key that allows and event to go through to the next lower layer.
        Useful when composing layers.