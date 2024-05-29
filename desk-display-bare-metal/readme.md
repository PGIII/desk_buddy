# Bare metal desk display

E-Paper display built using an ESP32S3 that can display various things such as the Weather.

## Build

```sh
cargo build
```

## Flash

```sh
cargo run
```

## Monitor

```sh
espflash monitor /dev/ttyUSB0
```

- Replace `dev/ttyUSB0` above with the USB port where you've connected the board. If you do not
  specify any USB port, `cargo-espflash`/`espflash` will print a list of the recognized USB ports for you to select
  the desired port.

## Additional information

For more information, check out:

- The [Rust on ESP Book](https://esp-rs.github.io/book/)
- The [ESP STD Embedded Training](https://github.com/esp-rs/std-training)
- The [esp-idf-hal](https://github.com/esp-rs/esp-idf-hal) project
- The [embedded-hal](https://github.com/rust-embedded/embedded-hal) project
- The [esp-idf-svc](https://github.com/esp-rs/esp-idf-svc) project
- The [embedded-svc](https://github.com/esp-rs/embedded-svc) project
- The [esp-idf-sys](https://github.com/esp-rs/esp-idf-sys) project
- The [Rust for Xtensa toolchain](https://github.com/esp-rs/rust-build)
- The [Rust-with-STD demo](https://github.com/ivmarkov/rust-esp32-std-demo) project

## Prerequisites

Linux/Mac users: Make sure you have the dependencies installed,that are mentiond in the [esp-idf install guide](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/get-started/linux-macos-setup.html#step-1-install-prerequisites). You **dont** need to manually install esp-idf, just its dependencies.

For detailed instructions see [Setting Up a Development Environment](https://esp-rs.github.io/book/installation/index.html) chapter of The Rust on ESP Book.

### Install Rust (with `rustup`)

If you don't have `rustup` installed yet, follow the instructions on the [rustup.rs site](https://rustup.rs)

### Install Cargo Sub-Commands

```sh
cargo install cargo-generate
cargo install ldproxy
cargo install espup
cargo install espflash
cargo install cargo-espflash # Optional
```

> [!NOTE]
> If you are running Linux then `libudev` must also be installed for `espflash` and `cargo-espflash`; this is available via most popular package managers. If you are running Windows you can ignore this step.

> ```
> # Debian/Ubuntu/etc.
> apt-get install libudev-dev
> # Fedora
> dnf install systemd-devel
> ```

> Also, the `espflash` and `cargo-espflash` commands shown below, assume that version `2.0` or
> greater.

### Install Rust & Clang toolchains for Espressif SoCs (with `espup`)

```sh
espup install
# Unix
. $HOME/export-esp.sh
```

> [!WARNING]
> Make sure you source the generated export file, as shown above, in every terminal before building any application as it contains the required environment variables.

See the [Installation chapter of The Rust on ESP Book](https://esp-rs.github.io/book/installation/index.html) for more details.

### Install Python3

You need a Python 3.7 or later installed on your machine.

- Linux, Mac OS X: if not preinstalled already, just install it with your package manager, i.e. for Debian systems: `sudo apt install python3`
- Windows: install it e.g. [from the official Python site](https://www.python.org/downloads/).

You'll also need the Python PIP and Python VENV modules. On Debian systems, you can install with:

- `sudo apt install python3-pip python3-venv`
