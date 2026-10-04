# Mapping and profiles {#mapeamento}

A **profile** says how what the phone sends becomes the controller the game receives: deadzones, curves, swapped buttons, turbo, gyro and vibration. In **Mapping** you edit everything with the controller drawn on screen.

## How to edit {#como-editar}

The screen has three columns: the **profile list**, the **controller** and the **properties**. Click a button, trigger or stick in the drawing to adjust it. Pick **Preview with: P1** (or another) to watch the controller move live while you adjust.

Changes are saved automatically and take effect **right away**, without reconnecting. **Ctrl+Z** undoes.

## Buttons {#botoes}

- **Map to**: swap a button for another, or for a trigger, or ignore it.
- **Turbo**: while the button is held, it toggles on and off 1 to 30 times per second.

## Sticks {#analogicos}

- **Inner deadzone**: the middle that reads as zero (removes jitter). **Outer**: the end of the travel, to reach the maximum before the edge. Drag the circle handles or use the sliders.
- **Shape**: circular or per axis.
- **Anti-deadzone**: starts the response above zero, for games that have their own deadzone.
- **Sensitivity** and **Curve**: linear, exponential or custom with four points you drag (or move with the keyboard arrows).
- **Invert** X or Y and **swap left and right**.

## Triggers {#gatilhos}

Deadzone, max point (from where it counts as 100%) and **digital mode** (on or off, nothing in between).

## Vibration {#vibracao}

In the **Vibration** tab, choose whether what the game asks for reaches the phone and with what intensity (0% to 200%). Above 100% it saturates at the maximum. On the dashboard, **Test vibration** (the card’s **…** menu) makes the phone vibrate for half a second.

## Manage profiles {#gerenciar-perfis}

- **New profile**, **Duplicate**, **Delete** (with confirmation; if the profile is in use, the app tells you how many devices and controllers use it).
- **Export** creates a `.padprofile` file to share, and **Import** opens one. The app checks the file before using it.
- **Restore default** puts the Default profile back to the original.
- The **Default** profile belongs to the app and can’t be edited: duplicate it to customize.
- In **Settings → Controllers** you choose the profile a new phone gets.
