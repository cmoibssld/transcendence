This project has been created as part of the 42 curriculum by jlepany, aguilleu, acolonne, cabarrou

Install cargo in 42: 

```
cat ~/.profile
export CARGO_HOME="/home/$USER/goinfre/.cargo"
export RUSTUP_HOME="/home/$USER/goinfre/.rustup"

$HOME/bin/setup_rustup.sh 2> $HOME/rustup_log

. "/home/$USER/goinfre/.cargo/env"
cat ~/bin/setup_rustup.sh
#!/bin/bash

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

. "$HOME/goinfre/.cargo/env"

rustup default stable
```
should install cargo in goinfre

if this error: 
```
error: rustup could not choose a version of cargo to run, because one wasn't specified explicitly, and no default is configured.
help: run 'rustup default stable' to download the latest stable release of Rust and set it as your default toolchain.
```
do
```
export CARGO_HOME="/home/$USER/goinfre/.cargo"
export RUSTUP_HOME="/home/$USER/goinfre/.rustup"
```

and uninstall by:
```
rustup self uninstall
```
or just delete the .cargo and .rustup directories
