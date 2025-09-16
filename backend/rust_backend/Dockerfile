FROM ubuntu:22.04

# Prevents interactive installation dialogs
ENV DEBIAN_FRONTEND=noninteractive

# Install essential packages and dependencies
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        curl \
        pkg-config \
        python3 \
        python3-pip \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install Rust using rustup
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y \
    && . "$HOME/.cargo/env"

# Add Rust to PATH
ENV PATH="/root/.cargo/bin:${PATH}"

# apt-get update
RUN apt-get update
#RUN apt-get install libpq-dev

# Install cargo-lambda
RUN pip3 install cargo-lambda

# Set working directory inside container
WORKDIR /rust_back_end

# Default command launch bash shell
CMD ["bash"]