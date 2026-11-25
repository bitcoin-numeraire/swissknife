# SwissKnife Desktop

The desktop client opens a SwissKnife dashboard in a native system webview. Enter the dashboard origin, then sign in with the account you use on that instance. For single sign-on, expand **Single sign-on** and enter the identity provider origin. The hosted Numeraire instance is preconfigured. Close the wallet window to choose a different instance.

Use HTTPS for remote deployments. A dashboard running directly on your computer can use HTTP on localhost or a loopback IP address. The client stores the last selected dashboard and login origins locally. The system webview stores the instance’s session and device data. Webview storage is separate from your regular browser, so restore any device-held identities using their recovery details. The instance runs the API, database and Lightning provider.

## Develop and package

Install the [Tauri desktop prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform, the workspace Rust toolchain and Node.js 24. Run the following commands from this directory.

```sh
cargo test --locked
npm exec --yes --package=@tauri-apps/cli@2.12.6 -- tauri dev
npm exec --yes --package=@tauri-apps/cli@2.12.6 -- tauri build -- --locked
```

For a local dashboard, the executable also accepts `--instance http://localhost:8080` and an optional `--login-origin https://login.example.com`.

The Desktop packages workflow produces an Intel and Apple Silicon universal macOS DMG, Windows x64 NSIS installer, and Linux x64 DEB and AppImage as pull request artifacts. Public distribution requires platform signing, macOS notarization and installer acceptance on the target systems.
