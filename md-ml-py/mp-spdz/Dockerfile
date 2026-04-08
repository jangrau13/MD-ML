FROM python:3.12-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    automake build-essential clang cmake git \
    libboost-dev libboost-filesystem-dev libboost-iostreams-dev \
    libboost-thread-dev libgmp-dev libntl-dev libsodium-dev \
    libssl-dev libtool \
    yasm texinfo \
    ca-certificates wget \
  && rm -rf /var/lib/apt/lists/*

# Clone MP-SPDZ
WORKDIR /build
RUN git clone --depth 1 https://github.com/data61/MP-SPDZ.git

WORKDIR /build/MP-SPDZ

# Setup submodules and deps (only once)
RUN make setup && mkdir -p /built

# Build spdz2k-party.x for each ring size.
# Use -Wno-unused-parameter to fix sse2neon.h errors on ARM.
# Only remove MP-SPDZ .o files between builds (not deps/) to avoid full rebuilds.
RUN echo 'MY_CFLAGS += -DRING_SIZE=5 -Wno-unused-parameter' > CONFIG.mine \
 && make -j$(nproc) spdz2k-party.x 2>&1 \
 && cp spdz2k-party.x /built/spdz2k-party-5.x \
 && find . -maxdepth 3 -name '*.o' -not -path './deps/*' -not -path './local/*' -delete

RUN echo 'MY_CFLAGS += -DRING_SIZE=10 -Wno-unused-parameter' > CONFIG.mine \
 && make -j$(nproc) spdz2k-party.x 2>&1 \
 && cp spdz2k-party.x /built/spdz2k-party-10.x \
 && find . -maxdepth 3 -name '*.o' -not -path './deps/*' -not -path './local/*' -delete

RUN echo 'MY_CFLAGS += -DRING_SIZE=32 -Wno-unused-parameter' > CONFIG.mine \
 && make -j$(nproc) spdz2k-party.x 2>&1 \
 && cp spdz2k-party.x /built/spdz2k-party-32.x \
 && find . -maxdepth 3 -name '*.o' -not -path './deps/*' -not -path './local/*' -delete

RUN echo 'MY_CFLAGS += -DRING_SIZE=64 -Wno-unused-parameter' > CONFIG.mine \
 && make -j$(nproc) spdz2k-party.x 2>&1 \
 && cp spdz2k-party.x /built/spdz2k-party-64.x

# ---------- Runtime image ----------
FROM python:3.12-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    libboost-filesystem-dev libboost-iostreams-dev \
    libboost-thread-dev libgmp-dev libntl-dev libsodium-dev \
    libssl-dev \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /mp-spdz

# Copy built executables, shared libs, and supporting files
COPY --from=builder /built/spdz2k-party-5.x .
COPY --from=builder /built/spdz2k-party-10.x .
COPY --from=builder /built/spdz2k-party-32.x .
COPY --from=builder /built/spdz2k-party-64.x .
COPY --from=builder /build/MP-SPDZ/libSPDZ.so .
COPY --from=builder /build/MP-SPDZ/local/lib/ /usr/local/lib/
COPY --from=builder /build/MP-SPDZ/Scripts/ Scripts/
COPY --from=builder /build/MP-SPDZ/Compiler/ Compiler/
COPY --from=builder /build/MP-SPDZ/compile.py .
RUN mkdir -p Player-Data && ldconfig
ENV LD_LIBRARY_PATH=/mp-spdz:/usr/local/lib

# Copy our benchmark programs
COPY programs/ Programs/Source/
COPY bench.py .
COPY entrypoint.sh .
RUN chmod +x entrypoint.sh

EXPOSE 5000 14000 14001

ENTRYPOINT ["./entrypoint.sh"]
