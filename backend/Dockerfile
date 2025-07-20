# AWS Lambda Rust Runtime
FROM public.ecr.aws/lambda/provided:al2023

# Install development tools
RUN dnf update -y && \
    dnf install -y gcc gcc-c++ make && \
    dnf clean all

# Install Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

# Add x86_64 target for Lambda (GitHub Actions runs on x86_64)
RUN rustup target add x86_64-unknown-linux-gnu

# Set working directory
WORKDIR ${LAMBDA_TASK_ROOT}

# Copy source code
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

# Build the application for x86_64
RUN cargo build --release --target x86_64-unknown-linux-gnu

# Copy the binary to the runtime directory as bootstrap
RUN cp target/x86_64-unknown-linux-gnu/release/backend ${LAMBDA_RUNTIME_DIR}/bootstrap

# Set the CMD to your handler
CMD ["bootstrap"]