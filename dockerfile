FROM ghcr.io/cross-rs/riscv64gc-unknown-linux-gnu:edge

RUN dpkg --add-architecture riscv64 && apt update && apt upgrade -y && apt install libasound2-dev:riscv64 libjack-dev:riscv64 -y
    
