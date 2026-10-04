# Latency: is the input delayed? {#latencia}

**Latency** is the time between touching the phone and the game reacting. VynPad measures each controller’s connection all the time and shows it in **Latency** and on the dashboard card.

## What the screen shows {#o-que-a-tela-mostra}

- **RTT** (p50 and p95): the round-trip time to the phone. p95 is what you feel in the worst moments.
- **Jitter**: how much the interval between sends varies. High jitter feels like stutter.
- **Loss**: how many packets didn’t arrive.
- **Packets/s**: the send rate (250 per second is normal).
- **On the server**: how long the PC takes to hand the input to the driver.
- **Quality**: Excellent, Good, Unstable or Poor, by the worst of the measures.

The chart shows each controller’s RTT in the slot colors, over 30 s, 2 min or 10 min windows. You can pause it, hide a controller from the legend and copy a text summary to ask for help.

## How to improve it {#como-melhorar}

1. Use **5 GHz Wi-Fi** and stay close to the router. It’s the biggest difference.
2. Turn off **Wi-Fi battery saving** on the phone (it lets the radio sleep between packets and raises jitter).
3. Close apps that use a lot of network on the PC and the phone (downloads, streaming).
4. Prefer the PC on a **cable** to the router: only the phone is on Wi-Fi.
5. In **Settings → Controllers**, the **250 Hz** send rate is more precise than 125 Hz; 125 Hz saves battery and network.

If RTT is good (under 20 ms) and the game still feels slow, the delay may be in the game or the monitor, not in the connection.
