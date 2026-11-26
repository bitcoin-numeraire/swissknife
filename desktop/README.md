# SwissKnife Desktop

The desktop client opens a SwissKnife dashboard in a native system webview. The launcher uses your first supported system language. Choose English, French, German, Spanish, Italian or Portuguese in **Language** to remember a different choice on this computer. Select **System language** to follow the device again. The dashboard has its own language preference.

Enter the dashboard origin, then sign in with the account you use on that instance. For single sign-on, expand **Single sign-on** and enter the identity provider origin. The hosted Numeraire instance is preconfigured. Close the wallet window to choose a different instance.

Use HTTPS for remote deployments. A dashboard running directly on your computer can use HTTP on localhost or a loopback IP address. The client stores the last selected dashboard and login origins locally. The system webview stores the instance’s session and device data. Sign in to the desktop client separately from your regular browser. The instance runs the API, database and Lightning provider.

## Develop and package

Install the [Tauri desktop prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform, the workspace Rust toolchain and Node.js 24. Run the following commands from this directory.

```sh
cargo test --locked
npm exec --yes --package=@tauri-apps/cli@2.12.6 -- tauri dev
npm exec --yes --package=@tauri-apps/cli@2.12.6 -- tauri build -- --locked
```

For a local dashboard, the executable also accepts `--instance http://localhost:8080` and an optional `--login-origin https://login.example.com`.

The Desktop packages workflow produces an Intel and Apple Silicon universal macOS DMG, Windows x64 NSIS installer, and Linux x64 DEB and AppImage as pull request artifacts. The workflow installs a test predecessor, upgrades it to the proposed package and removes the application on each native runner. It includes SHA-256 checksums for the Linux packages. Public distribution requires platform signing, macOS notarization and acceptance of the signed release candidates.
