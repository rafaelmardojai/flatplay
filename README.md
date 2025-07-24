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

# Some notes

- A lot of the code is almost directly borrowed from [`flatpak-vscode`](https://github.com/bilelmoussaoui/flatpak-vscode).
- Not all features are properly implemented yet. The focus so far has been just on getting something working to conveniently work on [Wordbook](https://github.com/mufeedali/Wordbook) while using anything that's not VS Code.
- I hope to improve it over time and I hope Bilal forgives me for the sins I've committed here.
- I'm not a Rust programmer.
