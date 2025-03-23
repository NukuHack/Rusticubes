To install Rust and use `cargo run` in the command prompt (CMD) on Windows, follow these steps:

**decided to make it easy**
 - just run the `install.bat` and in theory everything should work
 - you may be prompted to run the `download_tools.bat` if so the run it (might need admin for installing everything)
 - then just run the `run.bat` every time you want to launch the app

---

### **Step 1: Install Rust**
Rust provides an easy-to-use installer called `rustup`, which manages Rust installations. Here's how to install it:

1. **Download and Run the Installer**:
   - Open your browser and go to the official Rust installation page: [Rust-Website](https://www.rust-lang.org/tools/install).
   - Alternatively, open a terminal or CMD and run the following command:
     ```bash
     curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
     ```
     If you're on Windows, download and run the `rustup-init.exe` executable from the website.

2. **Follow the Installation Prompts**:
   - The installer will guide you through the process. By default, it installs Rust and its tools (like `cargo`) in your user directory.
   - When prompted, choose the default installation options unless you have specific requirements.

3. **Verify Installation**:
   After the installation is complete, verify that Rust and Cargo are installed by running:
   ```bash
   rustc --version
   cargo --version
   ```
   These commands should display the installed versions of the Rust compiler (`rustc`) and the package manager (`cargo`).

---

### **Step 2: Add Rust to Your System PATH**
On Windows, the `rustup` installer typically adds Rust to your system PATH automatically. However, if `cargo run` doesn't work after installation, you may need to manually add Rust to your PATH:

1. **Locate the Rust Installation Directory**:
   - By default, Rust installs its binaries in:
     ```
     C:\Users\<YourUsername>\.cargo\bin
     ```
     Replace `<YourUsername>` with your actual Windows username.

2. **Add to PATH**:
   - Open the Start menu, search for "Environment Variables," and select **Edit the system environment variables**.
   - In the System Properties window, click the **Environment Variables** button.
   - Under **System variables**, find the `Path` variable and click **Edit**.
   - Click **New** and add the path to the `.cargo\bin` directory:
     ```
     C:\Users\<YourUsername>\.cargo\bin
     ```
   - Click **OK** to save the changes.

3. **Restart CMD**:
   - Close and reopen your command prompt for the changes to take effect.

---

### **Step 3: Test `cargo run`**
Now that Rust and Cargo are installed, you can test `cargo run`:

1. **Create a New Rust Project**:
   Run the following command to create a new Rust project:
   ```bash
   cargo new hello_world
   ```
   This creates a new directory called `hello_world` with a basic Rust project structure.

2. **Navigate to the Project Directory**:
   ```bash
   cd hello_world
   ```

3. **Run the Project**:
   Use the `cargo run` command to compile and execute the program:
   ```bash
   cargo run
   ```
   You should see output similar to:
   ```
   Hello, world!
   ```

---

### **Troubleshooting**
If you encounter issues:
1. **Check PATH**: Ensure the `.cargo\bin` directory is correctly added to your PATH.
2. **Update Rust**: Run `rustup update` to ensure you have the latest version of Rust.
3. **Reinstall Rust**: If problems persist, uninstall Rust using `rustup self uninstall` and reinstall it.

---

By following these steps, you should be able to install Rust and use `cargo run` seamlessly in your command prompt. Let me know if you need further assistance!
