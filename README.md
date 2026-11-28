# Cinny desktop

<a href="https://github.com/cinnyapp/cinny-desktop/releases">
  <img alt="GitHub release downloads" src="https://img.shields.io/github/downloads/cinnyapp/cinny-desktop/total?style=social"></a>

Cinny is a matrix client focusing primarily on simple, elegant and secure interface. The desktop app is made with Tauri.

## Download

Installers for macOS, Windows and Linux can be downloaded from [Github releases](https://github.com/cinnyapp/cinny-desktop/releases). Releases are signed with a [Ed25519](https://ed25519.cr.yp.to/) public-key.

Operating System | Download
---|---
Windows | <a href='https://github.com/cinnyapp/cinny-desktop/releases/latest/download/Cinny_desktop-x86_64.msi'>Cinny.msi</a> · <a href='https://scoop.sh/#/apps?q=cinny'>Install via Scoop</a>
Windows ARM64 | <a href='https://github.com/cinnyapp/cinny-desktop/releases/latest/download/Cinny_desktop-aarch64.exe'>Cinny.exe</a>
macOS | <a href='https://github.com/cinnyapp/cinny-desktop/releases/latest/download/Cinny_desktop-universal.dmg'>Cinny.dmg</a> · <a href='https://github.com/cinnyapp/cinny-desktop/releases/latest/download/Cinny_desktop-universal.app.tar.gz'>Cinny.app</a>
Linux | <a href='https://github.com/cinnyapp/cinny-desktop/releases/latest/download/Cinny_desktop-x86_64.AppImage'>Cinny.AppImage</a> · <a href='https://flathub.org/apps/details/in.cinny.Cinny'>Flatpak</a>

Decoded public key:
> RWRflTUQD3RHFtn25QNANCmePR9+4LSK89kAKTMEEB4OKpOFpLMgc64z

To verify release files, you need to download [minisign](https://jedisct1.github.io/minisign/) tool and [decode](https://www.base64decode.org/) the *.sig* file before running:
>  minisign -Vm ***RELEASE_FILE.msi.zip*** -P RWRflTUQD3RHFtn25QNANCmePR9+4LSK89kAKTMEEB4OKpOFpLMgc64z -x ***Decoded_SINGATURE.msi.zip.sig***

## Desktop notifications

Choose a **Notification Mode** in Cinny's **Settings > Notifications**:
**Desktop + audio**, **Audio only**, **Desktop only**, or **No notifications**.
Existing desktop and audio preferences are preserved. Messages
and invitations can produce native notifications while Cinny is running, including
when its window is hidden in the tray. Existing room notification preferences and
the notification sound setting still apply.
Message toasts use the sender's profile picture when available. Images are loaded
through Cinny's authenticated media handling and cached locally for Windows;
missing or slow avatars fall back to the normal app icon.

The first new message alerts immediately. Further messages in the same conversation
stay quiet for five minutes. After that, another new message can trigger a reminder;
there are no timer-based reminders. Reading the conversation resets its cooldown.
Invitation bursts share a separate five-minute cooldown.

On Windows, clicking a visible toast restores Cinny and opens its conversation or
invitations. Clicking a notification after it moves to Notification Center is not
supported. Cinny registers its notification identity for the current Windows user
when sending a toast, so launching the EXE directly also works without an installer.
This registration uses Cinny's app name and a cached icon. Windows notification
settings and Do Not Disturb can suppress banners.

Use **Send test notification** below the notification mode to check native delivery
without waiting for a message or the cooldown. Submission errors appear beside the
button. After rebuilding, choose **Quit** from the tray menu before launching the
new EXE; launching again otherwise restores the existing process.

Notification regression tests run with `npm run test:notifications` from `cinny/`,
using the Node version specified in `.node-version`.
After a Cargo check/build, run `npm run test:desktop-acl` from the repository root
to verify that the generated release capability grants the desktop commands.

## Local development

Firstly, to setup Rust, NodeJS and build tools follow [Tauri documentation](https://v2.tauri.app/start/prerequisites/).

Now, to setup development locally run the following commands:
* `git clone --recursive https://github.com/cinnyapp/cinny-desktop.git`
* `cd cinny-desktop/cinny`
* `npm ci`
* `cd ..`
* `npm ci`

To build the app locally, run:
* `npm run tauri build`

To start local dev server, run:
* `npm run tauri dev`

## License
Cinny Project  
Copyright © 2024–present Ajay Bura  
https://cinny.in  

Cinny is licensed under the GNU Affero General Public License, 
Version 3 of the License (AGPL-3.0-only).
You may obtain a copy of the License at https://www.gnu.org/licenses/agpl-3.0.html
