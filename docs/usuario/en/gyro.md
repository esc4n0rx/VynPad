# Gyro: aim by moving the phone {#gyro}

The gyro uses the phone’s motion sensor to move a stick, like console games with motion aiming. Good for shooters and aiming games.

## Turn it on {#ligar}

In **Mapping → Gyro**, turn on **Turn the gyro on**. The phone starts sending motion along with the buttons (only if the device has the sensor).

## Two ways to use it {#resposta}

Under **How motion acts** you choose:

- **Speed (aiming)**: the stick moves *while* the phone turns and returns to center when it stops. This is how console motion aiming works.
- **Tilt (steering)**: the stick follows the phone's **angle** and **stays where it is**. Turn the phone like a steering wheel and the left stick follows, precisely. This is the right one for simulators and racing games (Euro Truck Simulator, for example).

Choosing **Tilt** sets the profile up as a wheel (left stick, X axis, turn like a steering wheel, no smoothing). You can change it afterwards. **Angle to full travel** is how many degrees take the stick to its maximum (45° by default: fewer degrees = more sensitive wheel). The **recenter** button now takes the phone's current position as the neutral point.

The angle comes from the gravity the phone measures, corrected by the gyroscope, so it copes well with vibration. If you hold the phone flat (like a tray), use the **Turn sideways** motion.

## When it acts {#quando-age}

- **Always**: the gyro moves the stick all the time.
- **While held**: only while you hold a button (for example LB). Good for fine aiming.
- **Toggle**: each tap on the button turns it on or off.

## Settings {#ajustes}

- **Target stick**: left or right (usually the right one, which aims the camera).
- **X and Y axis motion**: turn sideways, tilt up and down, or rotate like a wheel.
- **X and Y sensitivity**: how far the stick moves for each motion. Start at 1.0.
- **Invert** X or Y.
- **Gyro deadzone**: ignores small hand tremors.
- **Smoothing**: a filter that removes jitter without delaying fast movements. A lower **cutoff frequency** smooths more; a higher **fast-motion response** reduces the delay.
- **Recenter button**: resets the filter and turns off toggle mode.
- **Add to the stick**: the gyro adds to what the stick already does (with a limit). Off, the gyro replaces the stick while it acts.

## Tips {#dicas-gyro}

- Hold the phone firmly and use both hands.
- If the aim drifts by itself, raise the gyro deadzone or use the recenter button.
- Aim that is too slow or too fast is a **sensitivity** adjustment; delay is something else, see [Latency](#latencia).
