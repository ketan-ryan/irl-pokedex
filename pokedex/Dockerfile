# Cross compilation (along with cross build --release --target aarch64-unknown-linux-gnu) for raspi
# docker build -t aarch64-unknown-linux-gnu:latest .
# On pi: sudo apt install python3-onnxruntime
FROM debian:trixie

ENV DEBIAN_FRONTEND=noninteractive

# Base tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    clang libclang-dev llvm-dev build-essential pkg-config \
    curl \
    git \
    build-essential \
    cmake \
    gcc-aarch64-linux-gnu \
    g++-aarch64-linux-gnu \
    libc6-dev-arm64-cross \
    libssl-dev \
    pkg-config \
    python3 \
    python3-onnxruntime \
    qemu-user \
    qemu-user-static \
 && rm -rf /var/lib/apt/lists/*

# Enable arm64 architecture for dev libraries
RUN dpkg --add-architecture arm64 && apt-get update && apt-get install -y --no-install-recommends \
    libglib2.0-dev:arm64 \
    libgstreamer1.0-dev:arm64 \
    libgstreamer-plugins-base1.0-dev:arm64 \
    libssl-dev:arm64 \
 && rm -rf /var/lib/apt/lists/*

RUN mkdir -p /opt/onnxruntime && \
    curl -fL "https://github.com/microsoft/onnxruntime/releases/download/v1.24.2/onnxruntime-linux-aarch64-1.24.2.tgz" \
    -o /tmp/ort.tgz && \
    tar -xzf /tmp/ort.tgz -C /opt/onnxruntime --strip-components=1 && \
    rm /tmp/ort.tgz

ENV ORT_LIB_LOCATION=/opt/onnxruntime/lib

# Cross-build environment variables
ENV \
  CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
  CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
  CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++ \
  QEMU_LD_PREFIX=/usr/aarch64-linux-gnu \
  PKG_CONFIG_ALLOW_CROSS=1 \
  PKG_CONFIG_SYSROOT_DIR=/usr/aarch64-linux-gnu \
  PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig \
  RUST_TEST_THREADS=1 \
  ORT_OFFLINE=1