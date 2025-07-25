# Flatplay

Early days.

```bash
# Install into ~/.local/bin (or XDG_BIN_HOME)
just install

# Enable completions (optional, replace with your shell)
flatplay completions fish > ~/.config/fish/completions/flatplay.fish

flatplay --help

# cd into a project directory
# then simply run:
flatplay

# This will attempt to build and run the project.
```

## Integrate into editors

### Zed

To integrate flatplay in Zed you can define custom tasks and key bindings.

Add the following tasks to your `tasks.json` file (you can open it with the `zed: open tasks` action):

```json
{
  "label": "build & run flatpak",
  "command": "flatplay build-and-run"
},
{
  "label": "build flatpak",
  "command": "flatplay build"
},
{
  "label": "run flatpak",
  "command": "flatplay run"
},
{
  "label": "stop flatpak",
  "command": "flatplay stop"
},
{
  "label": "clean flatpak build",
  "command": "flatplay clean"
},
{
  "label": "update flatpak dependencies",
  "command": "flatplay update-dependencies"
},
{
  "label": "select flatpak manifest",
  "command": "flatplay select-manifest"
}
```

Next, add new bindings to your `keymaps.json` file (you can open it with the `zed: open keypam` action):

```json
{
  "context": "Workspace",
  "bindings": {
    "ctrl-alt-b": ["task::Spawn", { "task_name": "build & run flatpak" }],
    "ctrl-alt-r": ["task::Spawn", { "task_name": "run flatpak" }],
    "ctrl-alt-c": ["task::Spawn", { "task_name": "stop flatpak" }]
  }
},
```

Then you can run any of the tasks by pressing `Alt-Shift-T`, or use `Ctrl-Alt-B`, `Ctrl-Alt-R` or `Ctrl-Alt-C` to build & run, run or stop flatpaks.

# Some notes

- A lot of the code is almost directly borrowed from [`flatpak-vscode`](https://github.com/bilelmoussaoui/flatpak-vscode).
- Not all features are properly implemented yet. The focus so far has been just on getting something working to conveniently work on [Wordbook](https://github.com/mufeedali/Wordbook) while using anything that's not VS Code.
- I hope to improve it over time and I hope Bilal forgives me for the sins I've committed here.
- I'm not a Rust programmer.
