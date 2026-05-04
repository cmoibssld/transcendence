This project has been created as part of the 42 curriculum by jlepany, aguilleu, acolonne, cabarrou

Install cargo in 42: 

'''
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
'''
