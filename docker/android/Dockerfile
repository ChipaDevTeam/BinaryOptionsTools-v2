# Build the Android (aarch64) wheel with maturin + the Android NDK.
# Build from the repository root:
#   docker build -f docker/android/Dockerfile --target export -o dist/android .
FROM rust:slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    curl unzip build-essential pkg-config cmake \
    python3 python3-pip python3-venv \
    clang git \
    && rm -rf /var/lib/apt/lists/*

ENV VENV_PATH=/opt/venv
RUN python3 -m venv $VENV_PATH
ENV PATH="$VENV_PATH/bin:$PATH"
RUN pip install --upgrade pip && pip install maturin

# Android NDK setup
ENV ANDROID_NDK_VERSION=r26d
ENV ANDROID_SDK_ROOT=/opt/android-sdk
ENV ANDROID_NDK_HOME=$ANDROID_SDK_ROOT/ndk/android-ndk-$ANDROID_NDK_VERSION
ENV NDK_TOOLCHAIN=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64
ENV PATH="$NDK_TOOLCHAIN/bin:$PATH"

RUN mkdir -p $ANDROID_SDK_ROOT/ndk && \
    curl -L https://dl.google.com/android/repository/android-ndk-${ANDROID_NDK_VERSION}-linux.zip -o ndk.zip && \
    unzip -q ndk.zip -d $ANDROID_SDK_ROOT/ndk && \
    rm ndk.zip

RUN rustup target add aarch64-linux-android

ENV ANDROID_API_LEVEL=21
ENV CC_aarch64_linux_android=aarch64-linux-android21-clang \
    CXX_aarch64_linux_android=aarch64-linux-android21-clang++ \
    AR_aarch64_linux_android=llvm-ar \
    CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=aarch64-linux-android21-clang \
    BINDGEN_EXTRA_CLANG_ARGS_aarch64_linux_android="--sysroot=$NDK_TOOLCHAIN/sysroot"

# The NDK has no libpython; provide an empty stub so `-lpython3` resolves.
# Symbols are provided at runtime by the Python embedded in the Android app.
RUN mkdir -p /opt/python-stub && \
    echo "" | aarch64-linux-android21-clang -shared -nostdlib -x c - -o /opt/python-stub/libpython3.so
ENV RUSTFLAGS="-L /opt/python-stub -C link-arg=-Wl,--allow-shlib-undefined"

WORKDIR /app
COPY . .

WORKDIR /app/python
RUN maturin build --release --strip \
    --target aarch64-linux-android \
    -o /app/dist

# Export stage: contains only the built wheel(s)
FROM scratch AS export
COPY --from=builder /app/dist/*.whl /
