
# Rust Setup & Execution via Batch Files

**I made the development / running simple with these 3 batch files!**  
No manual configuration needed - just run the batch files in order.

---

## 🔥 Quick Start Guide

1. **Install Rust & Dependencies**  
   Run `install.bat` to set up Rust and required tools.  
   *Automatically handles PATH configurations and dependencies.*

2. **Install Additional Tools (if prompted)**  
   If you see a prompt, run `tools.bat` (may require admin privileges).  
   *Installs optional tools, or other dependencies.*

3. **Launch Your Rust App**  
   Every time you want to run your project, simply execute `run.bat`.  
   *Compiles and runs your Rust application with one click.*

---

## 📦 What Each Batch File Does

### `install.bat`
- Installs Rust using `rustup`
- Adds Rust to your system PATH
- Verifies installation success
- Checks for `tools.bat` dependencies

### `tools.bat` *(Admin Required)*
- Installs additional tools required for your project
- Handles the installation and configuration (not really)
- currently not maintained ... I just don't like it

### `run.bat`
- Compiles your Rust code with `cargo build`
- Runs your app with the executable
- Handles environment variables automatically

---

## 🛠️ Troubleshooting Tips

- **"Batch file not recognized"**  
  Ensure you're running the files as Administrator when prompted

- **Installation issues**  
  Delete the `.cargo` folder in your user directory and rerun `install.bat`

- **Compiler errors**  
  Check if `install.bat` was successfully executed

---

## 📝 Optional: Manual Setup Notes

If you prefer to understand the underlying steps:
1. Rust is installed via `rustup` in your user directory (`C:\Users\<YourName>\.cargo`)
2. PATH variables are automatically updated by `install.bat`
3. `run.bat` uses your project's `Cargo.toml` configuration

---

## 💡 Why Use These Batch Files?

- **No manual configuration**  
  Automatically handles PATH settings, dependencies, and environment variables

- **Cross-project compatibility**  
  Works with 'most' Rust projects out of the box

- **One-click workflow**  
  Eliminates repetitive setup steps every time you want to run your app

---
