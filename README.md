# Run or Focus for Umbriel

A small helper for [Umbriel](https://github.com/umbriel-uwu/umbriel) that provides keyboard-driven application switching similar to pinned apps in GNOME Dash-to-Dock, KDE Plasma, Unity, and the Windows taskbar.

Pressing one key can:

- focus an existing window
- cycle through multiple windows of the same app
- switch back to the previously focused window when only one match exists
- move to the correct workspace if needed
- launch the application if it is not already running


---

## Requirements

- Umbriel
- jq

---

## Installation

```bash
mkdir -p ~/.local/bin
cp ./rs ~/.local/bin/rs
chmod +x ~/.local/bin/rs
```

If you want the script to be available globally, add `~/.local/bin` to your `PATH`.

---

## Usage

```bash
rs [-t|--tag <tag>] [-a|--appid <appid>] [-T|--title <window_title>] [-i|--ignore <ignore_title_fragment>] [-c|--command <umbriel_msg_command>] [-n|--no-focuslast] [-- <launch_command...>]
```

You can match by:

- app ID only
- window title only
- both app ID and title
- app ID/title while ignoring a title fragment

Use `--` before the launch command.

### Arguments

| Argument | Description |
|---|---|
| `-t`, `--tag` | Workspace to switch to when the app is not running |
| `-a`, `--appid` | Application ID used by Umbriel |
| `-T`, `--title` | Window title or title fragment to match |
| `-i`, `--ignore` | Title fragment to exclude from matches |
| `-c`, `--command` | Optional `umbriel msg <command>` executed after launching and detecting the new window |
| `-n`, `--no-focuslast` | Keep the current app focused when there is exactly one matching window instead of switching back to the previously focused one |
| `-- <launch_command...>` | Command used to launch the application. Variables, arguments, and flags are supported. Use `sh -c "..."` for shell features like pipes, redirects, and `&&`. |

To inspect app IDs and titles:

```bash
umbriel windows --json | jq -r '.[] | [.app_id, .title] | @tsv'
```

---

## Behavior

- If the app is already focused and there are multiple matching windows, it cycles to the next one.
- If the app is already focused and there is only one matching window, it switches back to the previously focused window unless `-n/--no-focuslast` is set.
- If the app exists but is not focused, it focuses the matching window.
- If the app is not running, it switches to the target workspace (if `-t` is set), launches the app, and optionally runs `umbriel msg <command>` after the window appears.

This is the active implementation and preserves the same fast keyboard-driven workflow.

---

## Examples

Focus, cycle, or launch Konsole on workspace 1:

```bash
~/.local/bin/rs -t 1 -a org.kde.konsole -- konsole
```

Focus Nautilus or launch it on the current workspace:

```bash
~/.local/bin/rs -a org.gnome.Nautilus -- GSK_RENDERER=gl nautilus --new-window
```

Launch Midnight Commander in Alacritty or focus an existing match by title:

```bash
~/.local/bin/rs -T "mc [" -- alacritty -e mc
```

Run Alacritty or focus it while ignoring the Midnight Commander title fragment:

```bash
~/.local/bin/rs -a Alacritty -i "mc [" -- alacritty
```

Run a more complex shell command:

```bash
~/.local/bin/rs -a org.gnome.Nautilus -- sh -c "sleep 4 && GSK_RENDERER=gl nautilus --new-window"
```

Launch an app on workspace 2 and then run an extra Umbriel command:

```bash
~/.local/bin/rs -t 2 -a org.kde.konsole -c "set_prop,0.5" -- konsole
```

---

## Example keybindings

```ini
bind = SUPER, RETURN, spawn, ~/.local/bin/rs -t 1 -a org.kde.konsole -- konsole
bind = SUPER, e, spawn, ~/.local/bin/rs -a org.gnome.Nautilus -- GSK_RENDERER=gl nautilus
bind = SUPER, m, spawn, ~/.local/bin/rs -T "mc [" -- alacritty -e mc
bind = SUPER, 1, spawn, ~/.local/bin/rs -a Alacritty -i "mc [" -- alacritty
```

The script is designed to be used as a single-purpose launcher/focus binding.

---

## License

GPL-3.0
