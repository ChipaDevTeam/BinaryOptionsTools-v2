FROM rust:latest

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl unzip build-essential pkg-config cmake \
    python3 python3-pip python3-venv \
    clang git libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create a virtualenv and install maturin inside
ENV VENV_PATH=/opt/venv
RUN python3 -m venv $VENV_PATH
ENV PATH="$VENV_PATH/bin:$PATH"

RUN pip install --upgrade pip && pip install maturin

# Android NDK setup
ENV ANDROID_NDK_VERSION=r26d
ENV ANDROID_SDK_ROOT=/opt/android-sdk
ENV ANDROID_NDK_HOME=$ANDROID_SDK_ROOT/ndk/$ANDROID_NDK_VERSION
ENV PATH="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH"

RUN mkdir -p $ANDROID_SDK_ROOT/ndk && \
    curl -L https://dl.google.com/android/repository/android-ndk-${ANDROID_NDK_VERSION}-linux.zip -o ndk.zip && \
    # Verify SHA256 checksum (example hash for r26d linux, user should update if version changes)
    echo "59f6323b764c22143714b304c5520a7b48981440026f394474753065da4a6755  ndk.zip" | sha256sum -c - && \
    unzip ndk.zip -d $ANDROID_SDK_ROOT/ndk && \
    rm ndk.zip

# Add Android Rust targets
RUN rustup target add aarch64-linux-android

COPY . .

ENV CC_aarch64_linux_android=aarch64-linux-android21-clang
ENV CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=aarch64-linux-android21-clang

# Build .whl using maturin
CMD ["maturin", "build", "--release", "--target", "aarch64-linux-android", "--interpreter", "python3"]
