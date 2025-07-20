# AWS Lambda Rust Runtime for ARM64
FROM public.ecr.aws/lambda/provided:al2023-arm64

# Install development tools
RUN dnf update -y && \
    dnf install -y gcc gcc-c++ make && \
    dnf clean all

# Install Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

# Add ARM64 target for cross compilation
RUN rustup target add aarch64-unknown-linux-gnu

# Set working directory
WORKDIR ${LAMBDA_TASK_ROOT}

# Copy source code
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

# Build the application
RUN cargo build --release --target aarch64-unknown-linux-gnu

# Copy the binary to the runtime directory as bootstrap
RUN cp target/aarch64-unknown-linux-gnu/release/backend ${LAMBDA_RUNTIME_DIR}/bootstrap

# Set the CMD to your handler
CMD ["bootstrap"]