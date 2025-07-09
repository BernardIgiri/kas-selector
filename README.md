# KAS Selector

**KAS Selector** is a [Relm4](https://github.com/Relm4/Relm4) base application that allows users to assign shell scripts to [KDE](https://kde.org/) Activity lifecycle events (e.g., `started`, `activated`, `deactivated`, `stopped`). It provides a simple graphical interface for managing these script bindings per activity and event.

​	👉 [Download It Now](#-download)

## ✨ Features

* Automatically detects existing KDE activities.
* Supports assigning `.sh` scripts to each activity's lifecycle events.
* Displays activity names and events, not raw file paths.
* Handles validation, linking, and cleanup of associated script files.
* Designed for KDE Plasma 6.

## 🤩 Background & Motivation

KDE’s "Activity" feature is one of its most powerful but least known tools. It allows users to segment their workflows into distinct contexts (separate from virtual desktops) with different wallpapers, window rules, and app arrangements.

What’s even less known is that KDE supports **per-activity scripting**: you can run custom shell scripts when an activity is started, stopped, activated, or deactivated. However, this feature is largely undocumented and lacks any graphical interface for configuration.

**KAS Selector** exists to make this hidden power accessible. It provides a clean UI for linking scripts to activity events, lowering the barrier to using Activities as a true automation and workflow engine in KDE.

## ⚙️ Environment Variables

The following environment variables can be used to customize the behavior of the app:

| Variable                    | Description                                                                               | Default                                           |
| --------------------------- | ----------------------------------------------------------------------------------------- | ------------------------------------------------- |
| `KAS_ROOT`                  | Overrides the default root path where the script files are stored per activity and event. | `$HOME/.local/share/kactivitymanagerd/activities` |
| `KAS_SCRIPT_NAME`           | The filename of the script to assign (must be a valid `.sh` file).                        | `kas-script.sh`                                   |
| `LANGUAGE` or `LC_MESSAGES` | Used to determine the preferred UI language via Fluent localization system.               | System locale                                     |

## 🔨 Build Instructions

Ensure you have the following installed:

* Rust (version 1.88 or later)
* GTK4 development libraries
* CMake and other native build tools

Then run:

```bash
cargo build --release
```

## ▶️ Run

```bash
KAS_ROOT=/custom/path \
KAS_SCRIPT_NAME=startup.sh \
cargo run
```

## 📁 Directory Structure

Scripts are stored under:

```
~/.local/share/kactivitymanagerd/activities/<activity-id>/<event>/<script>
```

For example:

```
~/.local/share/kactivitymanagerd/activities/1234-uuid/activated/activity_script.sh
```

## 💡More KDE Tips

For more KDE Tips and Trick, especially with "Activities", checkout my blog post [Optimizing KDE Activities For Max Productivity!](https://yequalscode.com/posts/kde-productivity-tips)

## 💾 Download

Prebuilt `.deb` packages are available on the [GitHub Releases page](https://github.com/BernardIgiri/kas-selector/releases/latest).

To install:

```bash
wget https://github.com/BernardIgiri/kas-selector/releases/latest/download/kas-selector_0.X.0-1_amd64.deb
sudo dpkg -i kas-selector_0.X.0-1_amd64.deb
