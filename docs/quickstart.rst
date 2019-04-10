Quickstart
===============

1. Install the project using Cargo. This will place the
   built binaries in the ``~/.cargo`` directory.

    .. code-block:: bash
        :linenos:

        cargo install --git https://github.com/andrew-hardin/virtual-space-cadet.git

2. Verify the binaries are findable. If this fails, the
   ``~/.cargo`` directory needs to be added to the ``$PATH``.

    .. code-block:: bash
        :linenos:

        which spacecadet
        spacecadet --help

3. Download the demo matrix and layer files. The matrix file maps
   event codes to matrix locations. The layer file converts matrix
   locations to behaviors.

    .. code-block:: bash
        :linenos:

        wget https://github.com/andrew-hardin/virtual-space-cadet/raw/master/keyboards/vim_cursor/layers.json
        wget https://github.com/andrew-hardin/virtual-space-cadet/raw/master/keyboards/vim_cursor/matrix.json

3. Find your physical keyboard device under ``/dev/input``.
   This isn't always easy task. Running ``evtest`` can sometimes
   be helpful in determining which device is your keyboard.

4. Attach the ``spacecadet`` driver to your keyboard device and
   remap the keys using the demo matrix and layer files.

    .. code-block:: bash
        :linenos:

        spacecadet --device /dev/input/your-device \
                   --layer layers.json \
                   --matrix matrix.json

    .. TIP::
        You may encounter permissions problems. The path of
        least resistance is to run the application as root.

        An alternative long-term fix involves modifying permissions
        such that you can read the ``/dev/input/your-device``
        and write to ``/dev/uinput``.

5. With the ``spacecadet`` driver running, try typing on your
   physical keyboard - it should react normally. However, holding
   the space bar for longer than 150 milliseconds temporarily switches
   to a cursor layer. With space held, use ``hjkl`` to move the cursor
   left, down, up and right.

   For more information on this particular layout, read about the
   :ref:`vim cursor` layout.