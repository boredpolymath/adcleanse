# AdCleanse 🧹
### Your Personal Privacy Audit & Ad-Profile Scrubbing Console

**Created by Bored Polymath Studios**

Welcome to **AdCleanse**! Have you ever wondered why you see the ads you do? Social networks and advertising platforms quietly build a detailed profile about you, tracking your interests, habits, and the places you visit. They use this invisible "algorithmic persona" to target ads. 

**AdCleanse** is a free, privacy-first desktop app that puts you back in control. It acts like a microscope for your digital footprint, letting you easily see what these platforms think they know about you, and giving you the power to automatically wipe that data clean.

---

## What Does AdCleanse Do? 🌟

AdCleanse is built for everyone—you don't need to be a tech expert to use it! Here is what it can do for you:

1. **Reveal Your Hidden Profile:** See the exact interest categories and tracking tags that advertising platforms have assigned to you.
2. **Track Changes Over Time:** Watch how your ad profile changes (or "drifts") as you browse the web.
3. **Automated Scrubbing:** Set up easy rules to automatically delete specific topics. Don't want to be targeted for "Fast Food" or "Mortgage Loans"? AdCleanse will clean it up for you in the background.
4. **Partner Opt-Out:** See which third-party companies and data brokers have uploaded your information, and revoke their access with a single click.

---

## Our Promise to Your Privacy 🔒

We built AdCleanse because we believe your data belongs to you. Our core principles are simple:

* **Zero Cloud Tracking:** AdCleanse runs entirely on your own computer. There are no external servers, no hidden analytics, and no crash reporters. We don't track you. Period.
* **Bank-Grade Security:** Your login sessions are stored safely in your operating system's native secure vault (like Apple Keychain or Windows Credential Manager). Your private data never touches your hard drive without strong AES-256 encryption.
* **Offline-First:** All your historical data and settings are saved locally. You only need the internet when AdCleanse is actively communicating with the advertising platforms on your behalf.
* **Invisible & Lightweight:** AdCleanse can run quietly in your system tray, using minimal memory and resources, so it won't slow down your computer.

---

## We Want You! (How to Contribute) 🤝

AdCleanse is an open-source community project, and we would absolutely love your help to make it better! You **do not** need to be a programmer to contribute. Here is how you can join the cause:

### For Non-Technical Contributors 🎨✍️
* **Spread the Word:** Tell your friends and family about digital privacy and share AdCleanse!
* **Translate:** Help us translate the app into different languages so people around the world can protect their privacy.
* **Test the App:** Use AdCleanse and let us know if you find any bugs or confusing screens. Feedback is incredibly valuable.
* **Design & UX:** If you have an eye for design, we'd love help making our dark-mode interface even more beautiful and intuitive.

### For Technical Contributors 💻🛠️
* **Rust Developers:** Help us optimize our native backend, improve our encrypted SQLite database, or tighten our zero-telemetry network engine.
* **Frontend Developers:** Our UI is built with clean HTML5, CSS3, and Vanilla JavaScript. Help us build new responsive dashboards or interactive data visualizations.
* **Security Researchers:** Try to break it! We welcome audits of our IPC bridge, our Tauri capabilities, and our data handling practices. Check out `SECURITY.md` for our bug bounty and disclosure guidelines.

Please read our [CONTRIBUTING.md](CONTRIBUTING.md) to get started!

---

## Technical Overview for Developers ⚙️

Under the hood, AdCleanse is a hybrid native application built with the **Tauri (v2)** framework.

* **Backend Core:** Written in memory-safe **Rust**. Handles network requests, background polling, local SQLite encryption (via SQLCipher), and OS Keyring integration.
* **Frontend:** A lightweight, framework-free Vanilla JS / CSS3 webview that communicates with the Rust backend via a strictly typed Inter-Process Communication (IPC) bridge.

### Development Setup

1. **Install Prerequisites:**
   * **Rust Toolchain:** `rustc` and `cargo` (1.75+).
   * **Tauri CLI:** `cargo install tauri-cli --version "^2.0"`.
   * **OS Dependencies:** Xcode Command Line Tools (macOS), C++ Build Tools (Windows), or `libwebkit2gtk-4.1-dev` (Linux).
2. **Run in Browser (UI Only):** 
   You can preview the interface using the mocked IPC by simply opening `ui/index.html` in your browser.
3. **Run Full Desktop App (Dev Mode):** 
   ```bash
   cargo tauri dev
   ```
4. **Build for Release:**
   ```bash
   cargo tauri build
   ```

---

## Licensing ⚖️
AdCleanse is dual-licensed under either the **Apache License, Version 2.0** or the **MIT License**, at your option. This ensures the project remains free and permissive.

## Trademark & Legal Disclaimer ⚠️
**AdCleanse** is an independent, local open-source privacy audit utility authored by Bored Polymath Studios. AdCleanse is not affiliated with, sponsored by, authorized by, or endorsed by Meta Platforms, Inc., Facebook, Instagram, or any of their affiliates or subsidiaries. All product names, trademarks, and registered trademarks cited in documentation or UI are property of their respective holders and are used solely for identification and interoperability purposes.
