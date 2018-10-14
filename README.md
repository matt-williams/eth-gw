# eth-gw

Serverless WebAssembly runtime, built on Ethereum.

## Build

### Setup

```
sudo apt-get install build-essential pkg-config libssl-dev
curl https://sh.rustup.rs -sSf | sh
source $HOME/.cargo/env
rustup toolchain install nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
cargo install --git https://github.com/alexcrichton/wasm-gc
```

### Server

```
cargo run
```

### Serverless functions

See [functions].

```
export ENS_ADDR=adb9e045ff13e72662d541eb334c59f4634ef8b0
```

## Install

```
sudo apt-get install docker.io
sudo usermod -aG docker $USER
```

### DNS

(Should really be containerized)

```
sudo apt-get install bind
sudo tee -a /etc/bind/named.local >/dev/null <<EOF
zone "eth-gw.uk.to" {
        type master;
        allow-transfer {none;};
        file "/etc/bind/db.eth-gw.uk.to";
};
EOF
sudo tee /etc/bind/db.eth-gw.uk.to >/dev/null <<EOF
$TTL    60
@       IN      SOA     eth-gw.uk.to. admin@eth-gw.uk.to. ( 1 3600 3600 3600 3600 ) ;
@       IN      NS      ns
@       IN      A       54.163.19.66
ns      IN      A       54.163.19.66
*       IN      A       54.163.19.66
EOF
sudo service bind9 reload
```

##

```
docker run -d --name geth -p 127.0.0.1:8545:8545 -p 30303:30303 ethereum/client-go:alpine --rpc --rpcaddr "0.0.0.0"
docker run -d --name ipfs -p 4001:4001 -p 127.0.0.1:5001:5001 ipfs/go-ipfs:latest
```
