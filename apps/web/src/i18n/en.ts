import type { Dictionary } from "./pt-BR";

export const en: Dictionary = {
  meta: {
    title: "VynPad: your PC's controller, in your pocket",
    description:
      "Turn your phone into a virtual Xbox controller for Windows. Local Wi-Fi, low latency, rumble, gyro and a layout made your way. No account, no cloud.",
    ogAlt: "VynPad, your phone as a PC controller",
  },
  nav: {
    label: "Main navigation",
    features: "Features",
    how: "How it works",
    tech: "Engineering",
    download: "Download",
    faq: "FAQ",
    menuOpen: "Open menu",
    menuClose: "Close menu",
    home: "VynPad, home page",
  },
  theme: {
    label: "Theme",
    system: "Auto",
    dark: "Dark",
    light: "Light",
  },
  locale: { label: "Language", pt: "Português", en: "English" },
  hero: {
    eyebrowVersion: "Version {version}",
    eyebrowSoon: "First release coming soon",
    title: "Your PC's controller is in your pocket.",
    lead: "VynPad turns your phone into a virtual Xbox controller for Windows. Over your home Wi-Fi, with low latency, rumble, gyro and a layout designed your way.",
    ctaDownload: "Download for {os}",
    ctaDownloadGeneric: "Download VynPad",
    ctaHow: "See how it works",
    note: "No account. No cloud. It all happens on your network.",
    scene: {
      player: "Player {n}",
      desktopAlt:
        "The VynPad window on Windows with the A063 phone connected as Player 1, with 8 ms latency and an excellent connection.",
      phoneAlt:
        "The on-screen controller on an Android phone: sticks, D-pad, A, B, X and Y buttons, bumpers and triggers, in use.",
      phoneLabel: "On-screen controller on the phone",
      packetLabel: "Packets encrypted over Wi-Fi",
      caption: "Real VynPad screenshots on Windows and Android",
    },
  },
  proof: {
    label: "Technical facts",
    items: [
      "UDP at up to 250 Hz",
      "TLS with pinned certificate",
      "ChaCha20-Poly1305",
      "Rust core",
      "No account, no cloud",
    ],
  },
  how: {
    eyebrow: "How it works",
    title: "From download to your first game in a few minutes.",
    steps: [
      {
        title: "Install on your PC",
        body: "VynPad sets up the virtual controller driver and opens the firewall only for your local network. A five-step wizard handles it all.",
      },
      {
        title: "Pair with a QR",
        body: "Open the app on your phone and point it at the code on the PC screen. Or type a 6-digit PIN. The phone remembers the PC's identity and talks only to it.",
      },
      {
        title: "Play",
        body: "Games see an ordinary Xbox 360 controller. Nothing to configure inside each game.",
      },
    ],
    qrLabel: "Illustrative example of a pairing code",
    pinLabel: "or use the PIN",
    wizardSteps: ["Welcome", "Driver", "Firewall", "Pair", "Done"],
  },
  features: {
    eyebrow: "Features",
    title: "Everything a controller needs. And what only a phone has.",
    latency: {
      title: "Latency you can't feel",
      body: "The server processes each packet in under 0.1 ms and the button shows up in XInput in under 1 ms, measured on loopback. The end-to-end goal on 5 GHz Wi-Fi is under 10 ms, and the 240 fps camera measurement is under way.",
      chartLabel: "Illustrative chart of a controller's latency over time",
      p99: "server p99",
      goal: "End-to-end goal",
    },
    players: {
      title: "Up to 4 controllers at once",
      body: "Each phone gets a slot with its own color. Swap places, pause or disconnect with one click.",
    },
    gyro: {
      title: "Gyro for aiming or steering",
      body: "Turn the phone like a steering wheel and the stick follows the angle, ideal for simulators and racing. Or use motion aiming in shooters.",
      modeTilt: "Tilt",
      modeRate: "Speed",
      angle: "Angle",
    },
    rumble: {
      title: "The game's rumble, in your hand",
      body: "The game's rumble becomes phone vibration, with adjustable strength. When the game stops, the vibration stops with it.",
    },
    layout: {
      title: "Your layout, your game",
      body: "Move, resize and hide every button. Use your game's banner as the background, with the accent color taken from the image, and keep one look per game.",
      names: ["Racing", "Shooter", "Retro"],
    },
    privacy: {
      title: "Private by default",
      body: "No account, no cloud, no ads. Pairing pins the PC's certificate on the phone and input travels encrypted. Your layout images never leave the device.",
    },
  },
  tech: {
    eyebrow: "Engineering",
    title: "A lean protocol, written to be fast.",
    lead: "Input goes through no middlemen. It's a controller state in an encrypted UDP packet, delivered to the Rust server and translated for the virtual controller driver.",
    points: [
      {
        title: "Instantly, no clock",
        body: "Every change goes out immediately, up to 250 times per second, and the state repeats at 10 Hz as a heartbeat.",
      },
      {
        title: "Encrypted end to end",
        body: "The control channel uses TLS with the PC's certificate pinned at pairing. Input goes over UDP with ChaCha20-Poly1305 and a nonce that never repeats.",
      },
      {
        title: "No stuck buttons",
        body: "If the Wi-Fi drops for 250 ms, the virtual controller releases everything on its own. The session comes back without you touching anything.",
      },
    ],
    packet: {
      title: "Anatomy of a packet",
      header: "Header",
      headerFields: "magic · version · type · session · seq",
      payload: "Controller state",
      payloadFields: "buttons · triggers · sticks · gyro",
      tag: "Authentication",
      bytes: "{n} bytes",
      encrypted: "encrypted",
    },
    table: {
      title: "What we measured",
      rows: [
        { label: "Server processing (p99)", value: "0.039 ms" },
        { label: "UDP packet to XInput (p99, loopback)", value: "0.58 ms" },
        { label: "CPU with 4 controllers", value: "1.5 %" },
        { label: "Memory in the system tray", value: "23 MB" },
      ],
      note: "Measured on 2026-10-04, on Windows 11, with the real driver. Loopback: no Wi-Fi in the middle. End-to-end latency with the phone is still being measured.",
    },
  },
  download: {
    eyebrow: "Download",
    title: "Download VynPad",
    lead: "The server runs on Windows and the controller is the Android app. Install both and pair.",
    recommended: "Recommended for your system",
    windows: {
      name: "VynPad for Windows",
      requirements: "Windows 10 or 11, 64-bit",
      button: "Download the installer",
      altMsi: "MSI installer (for organizations)",
      warningTitle: "Windows may show a warning the first time",
      warningBody:
        "The installer isn't code-signed yet, so Windows SmartScreen may show “Windows protected your PC”. That doesn't mean the file is unsafe: click “More info” and then “Run anyway”. To be sure the file is the original, compare the SHA-256 below with your download's. The warning goes away as the installer builds reputation.",
    },
    android: {
      name: "VynPad for Android",
      requirements: "Android 8.0 or newer",
      play: "Get it on Google Play",
      soon: "Coming soon to Google Play",
      soonNote: "We're finishing testing before publishing.",
    },
    linux: {
      name: "VynPad for Linux",
      requirements: "Server for glibc-based distributions",
      soon: "Coming soon",
      soonNote: "It's on the roadmap, after Windows and Android.",
    },
    meta: {
      version: "Version",
      published: "Published",
      size: "Size",
      sha: "SHA-256",
      copy: "Copy",
      copied: "Copied",
      notes: "See what's new",
    },
    verify:
      "Check the SHA-256 after downloading to be sure the file is the one that was published.",
    empty: "The first public release hasn't been published yet. Check back soon.",
    error: "We couldn't check the latest version right now. Please try again in a moment.",
    unavailable: "That download isn't available right now.",
    prerelease: "Pre-release",
  },
  faq: {
    eyebrow: "Frequently asked questions",
    title: "What people usually ask.",
    items: [
      {
        q: "Do I need an account or the internet?",
        a: "No. The phone and the PC only need to be on the same local network. VynPad has no account, cloud or ads.",
      },
      {
        q: "Which games work?",
        a: "Any game that accepts an Xbox controller (XInput), which covers the vast majority of PC games. Games that only accept keyboard and mouse don't apply.",
      },
      {
        q: "Is it safe?",
        a: "The phone pins the PC's certificate at pairing and input travels encrypted. Still, only pair on networks you trust. Some online anti-cheats may not accept virtual controllers: check the game's rules.",
      },
      {
        q: "What about latency in practice?",
        a: "It depends on your network. Use 5 GHz Wi-Fi and, if you can, connect the PC to the router by cable. The app shows each controller's latency in real time.",
      },
      {
        q: "Does it work on iPhone, Linux or macOS?",
        a: "Not yet. The server runs on Windows and the controller is an Android app. Linux is on the roadmap; iPhone and macOS have no date.",
      },
      {
        q: "What does the gyro do?",
        a: "It can move a stick in two ways: by speed, for aiming, or by tilt, to turn the phone like a steering wheel in simulators and racing games.",
      },
    ],
  },
  cta: {
    title: "Ready to play with what's already in your pocket?",
    body: "Install on your PC, pair in seconds and start.",
    button: "Go to download",
  },
  footer: {
    tagline: "Your phone as your PC's controller.",
    product: "Product",
    legal: "Legal",
    privacy: "Privacy",
    changelog: "What's new",
    source: "Code on GitHub",
    rights: "© {year} VynPad. All rights reserved.",
  },
  privacy: {
    title: "Privacy policy",
    updated: "Updated on {date}",
    intro:
      "VynPad turns your phone into a controller for VynPad on your PC. This policy describes what the apps and this website do with your data.",
    sections: [
      {
        title: "We don't collect data",
        body: "The VynPad apps have no account, don't send information to us or to third parties, and use no analytics, advertising or crash-reporting tools.",
      },
      {
        title: "Network",
        body: "The app talks only to the PC you paired, on your local network. The control connection is encrypted with TLS and authenticated by the PC's certificate pinned at pairing, and input travels encrypted.",
      },
      {
        title: "Windows app",
        body: "VynPad on Windows keeps its database, controller profiles and pairings on your computer, in %APPDATA%\\VynPad, and sends no telemetry. The only internet connection is the update check, made against GitHub (the project's releases), which you can turn off under Settings, About. As with any download, GitHub receives your IP address. The app never installs an update by itself.",
      },
      {
        title: "Data stored on your device",
        body: "The pairing (the token is protected by the Android Keystore), your settings, your controller layouts and the images you choose as backgrounds. Images stay on the device and leave it only if you export a layout. Cloud backup and device-to-device transfer are turned off. Uninstalling the app deletes everything.",
      },
      {
        title: "Android app permissions",
        body: "Camera: used only to read the pairing QR code; no image is recorded or sent, and PIN pairing works without it. Vibration: game rumble and touch feedback. Network and Wi-Fi: talking to the PC and discovering it on the local network. Notifications and foreground service: they show the “Connected as P1” notice while a session is active, so Android doesn't close the connection to the PC; everything is local and nothing is sent. Sensors: gyroscope and accelerometer. They go to the PC only while the gyro is turned on there; for the tilt steering wheel, the accelerometer reading is used only on the device and the PC receives just the stick position. Picking a background image uses the system photo picker and doesn't ask for storage access.",
      },
      {
        title: "This website",
        body: "The website uses no tracking cookies or analytics tools. We only store, in your browser, the theme and language preferences you choose. When you download an installer, you are taken to the file published on GitHub, which has its own privacy policy.",
      },
      {
        title: "Children",
        body: "The apps are not directed at children and don't collect data from anyone.",
      },
      {
        title: "Changes and contact",
        body: "If this policy changes, the date above will be updated. Questions and requests can be sent to {email}.",
      },
    ],
  },
  changelog: {
    title: "What's new",
    lead: "The published VynPad versions, newest first.",
    empty: "There are no published versions yet.",
    error: "We couldn't load the versions right now. Please try again in a moment.",
    published: "Published on {date}",
    prerelease: "Pre-release",
    noNotes: "No notes for this version.",
    github: "View on GitHub",
  },
  notFound: {
    title: "Page not found",
    body: "The address doesn't exist or has moved.",
    back: "Back to home",
  },
  a11y: { skip: "Skip to content" },
};
