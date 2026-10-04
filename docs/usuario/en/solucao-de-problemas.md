# Troubleshooting {#solucao-de-problemas}

If nothing here solves it, go to **Settings → About → Export diagnostics**: the app creates a `.zip` file (without passwords, tokens or keys) for you to send to support.

## The phone can’t find the PC {#o-celular-nao-encontra-o-pc}

1. Check that the phone and the PC are on the **same Wi-Fi network** (no guest networks, which isolate devices).
2. In **Settings → Network**, see the PC’s address and use **Test connection**.
3. If the firewall is blocking, click **Fix** (see [The firewall is blocking](#o-firewall-bloqueia)).
4. If the network is set to **public**, change it to private (see [Public network](#rede-publica)).
5. VPNs, WSL and Hyper-V create virtual networks that confuse discovery. In **Settings → Network**, interfaces marked **Virtual** aren’t used.
6. As a last resort, type the PC’s IP on the phone (manual connection).

## The firewall is blocking {#o-firewall-bloqueia}

Windows must let the phone in on VynPad’s ports. In **Settings → Network → Windows Firewall**, click **Fix** (or **Allow connections**, in the introduction). Windows asks for administrator permission once and the app creates the rules only for **Private** and **Domain** networks. If you decline the permission, you can try again.

## Public network {#rede-publica}

On a network marked **public**, Windows blocks incoming connections, and VynPad’s rules don’t apply to it. Open the network settings (the app has the button) and change the network to **private**.

## ViGEmBus doesn’t show up {#vigembus}

Without **ViGEmBus**, VynPad can’t create controllers. See its state in **Settings → Controllers** (or the **Driver** step of the introduction):

- **Not installed**: use **Download ViGEmBus**, install it and come back. The app notices on its own.
- **Outdated**: install the version shown or a newer one.
- **Installed but stopped**: restart the PC or the “ViGEmBus” service in Windows Services.

## The game doesn’t recognize the controller {#o-jogo-nao-reconhece-o-controle}

- Confirm that the controller shows as connected on the dashboard and that the buttons move the drawing.
- Test in **joy.cpl** (Set up USB game controllers in Windows). If it works there, the problem is the game.
- Some games only accept the controller if it’s on **before** the game opens.
- Windows only sees **four XInput controllers** in total, counting physical and virtual ones. Turn off a controller you don’t use.
- Old games (DirectInput) may need a translator like x360ce.

## Steam Input duplicates the controller {#steam-input}

Steam can see the virtual controller and also the one the game sees, and the game gets two. In **Steam → Settings → Controller → General Controller Settings**, turn off **Xbox Configuration Support**, or in the game, choose **Disable Steam Input** in its properties.

## Delayed input {#input-atrasado}

See the [Latency](#latencia) page: use 5 GHz Wi-Fi, turn off Wi-Fi battery saving on the phone and check RTT and loss in **Latency**.

## Something went wrong {#err-internal}

An unexpected error. Try again. If it continues, export the diagnostics (**Settings → About**) and send it to support: the file shows what happened.

## A value is not valid {#err-invalid-input}

A field got a value outside what’s allowed (a port below 1024, for example). The message says which one. Fix the value and try again.

## Feature unavailable {#err-unsupported}

The action only exists on Windows (the ViGEmBus driver and the Windows firewall, for example). It isn’t available on this system.

## Item not found {#err-not-found}

The device, profile or controller you used no longer exists (someone removed it or the connection ended). Refresh the screen and try again.

## The Default profile can’t be edited {#err-builtin-profile}

The **Default** profile belongs to the app and stays fixed. Duplicate it (profile menu → **Duplicate**) and edit the copy. To go back to the original, use **Restore default**.

## Profile in use {#err-profile-in-use}

The profile you want to delete is being used by connected devices or controllers. The app shows how many and asks for confirmation; if you confirm, they switch to the **Default** profile.

## Invalid profile file {#err-import-invalid}

The `.padprofile` file is corrupted, too large or not a VynPad profile. Ask the person who exported it for another one, or export again.

## No local network {#err-no-network}

The PC isn’t on any local network, so there’s no way for the phone to reach it. Connect the PC to Wi-Fi or a wired network and try again.

## Windows permission declined {#err-cancelled}

You closed or declined Windows’ administrator permission request. The action wasn’t done. Try again and click **Yes** when Windows asks.

## Couldn’t update {#err-update-failed}

VynPad couldn’t reach the update server (GitHub), download the new version, or install it. Check that the PC has internet and try again in **Settings → About → Check for updates**. If it keeps failing, download the latest installer from the project’s releases page and install it over the current one: your pairings and profiles stay saved.

VynPad never installs an update on its own. With controllers connected it also asks for your confirmation, because updating closes the app for a moment.
