<h1 align="center">rusTry</h1>
<p align="center">Behold corrosion...</p>

---

### 🚀 A Rusty Experiment 🦀
A playful repository for learning Rust by building a game (no promises, but let’s see how far the rust spreads!).

---

## 🛠️ What's Inside?
- **Learning playground**: Small Rust projects for mastering concepts.
- **Game prototype**: WIP experiments with movement, visuals, and sounds.
- **Batch scripts**: `install.bat`, `run.bat`, and `tools.bat` for easy setup.

---

## 🔥 Quick Start Guide

1. **Install Rust & Dependencies**  
   Run `install.bat` to set up Rust and required tools.  
   *Automatically handles PATH configurations and dependencies.*

2. **Install Additional Tools (if prompted)**  
   If you see a prompt, run `tools.bat` (may require admin privileges).  
   *Currently not maintained; optional for advanced setups.*

3. **Launch Your Rust App**  
   Every time you want to run your project, simply execute `run.bat`.  
   *Compiles and runs your Rust application with one click.*

---

## 📦 What Each Batch File Does

### `install.bat`
- Installs Rust using `rustup`.
- Adds Rust to your system PATH.
- Verifies installation success.
- Checks for `tools.bat` dependencies.

### `tools.bat` *(Admin Required)*
- Installs additional tools required for your project.
- Currently not maintained; optional.

### `run.bat`
- Compiles your Rust code with `cargo build`.
- Runs your app with the executable.
- Handles environment variables automatically.

---

## 🛠️ Troubleshooting Tips

- **"Batch file not recognized"**  
  Ensure you're running the files as Administrator when prompted.

- **Installation issues**  
  Delete the `.cargo` folder in your user directory (`C:\Users\<YourName>\.cargo`) and rerun `install.bat`.

- **Compiler errors**  
  Check if `install.bat` was successfully executed.

---

## 📝 Optional: Manual Setup Notes

If you prefer to understand the underlying steps:
1. Rust is installed via `rustup` in your user directory (`C:\Users\<YourName>\.cargo`).
2. PATH variables are automatically updated by `install.bat`.
3. `run.bat` uses your project's `Cargo.toml` configuration.

---

## 💡 Why Use These Batch Files?

- **No manual configuration**  
  Automatically handles PATH settings, dependencies, and environment variables.

- **Cross-project compatibility**  
  Works with most Rust projects out of the box.

- **One-click workflow**  
  Eliminates repetitive setup steps every time you want to run your app.

---

## 🏗️ Roadmap
Here's what I'm aiming for (but no guarantees!):
1. **Phase 1**: Rust fundamentals (variables, ownership, macros).
2. **Phase 2**: Basic game mechanics (movement, collision).
3. **Phase 3**: Add "corrosion" flair (visuals, sound effects).

---

## 🚨 Current Status
> **🚧 Under construction**  
This is a learning project. Expect bugs, unfinished features, and lots of experimentation.

---

## 📝 License
MIT License - Feel free to borrow, tinker, or get inspired!

---

### 🌟 Future Dreams
![Game_image](/resources/happy-tree.png)  
*Maybe one day this will be a playable game...*

---

### 🛠️ Tech Stack
- **Rust**: Memory-safe, performant chaos.
- **Game crates**: `wgpu`, `winit`, `log`, `cgmath`.
- **Batch files**: Automation for the win.

---
