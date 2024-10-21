# Stage 1: Build
FROM rust:latest AS build

# Install dependencies
RUN apt-get update && apt-get install -y curl git build-essential clang libclang-dev

# Install NVM and Node.js
RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.3/install.sh | bash && \
    export NVM_DIR="/root/.nvm" && \
    [ -s "$NVM_DIR/nvm.sh" ] && \ 
    . "$NVM_DIR/nvm.sh" && \
    nvm install 18.18.0 && \
    nvm use 18.18.0 && \
    nvm alias default 18.18.0 && \
    ln -sf "$NVM_DIR/versions/node/v18.18.0/bin/node" /usr/local/bin/node && \
    ln -sf "$NVM_DIR/versions/node/v18.18.0/bin/npm" /usr/local/bin/npm && \
    ln -sf "$NVM_DIR/versions/node/v18.18.0/bin/npx" /usr/local/bin/npx

# Install Just (for build scripts)
RUN cargo install just

# Clone the Convex backend repository
RUN git clone https://github.com/get-convex/convex-backend.git /convex

WORKDIR /convex

# Install npm dependencies in the scripts folder
RUN npm install --prefix scripts

# Install Rush dependencies
RUN just rush install

# Build the convex-local-backend binary
RUN cargo build --release -p local_backend --bin convex-local-backend

# Stage 2: Final runtime
FROM ubuntu:22.04

# Install dependencies
RUN apt-get update && apt-get install -y libclang-dev libstdc++6 libc6

# Create a directory for the application
WORKDIR /usr/local/bin

# Copy the built binary from the build stage to the correct location
COPY --from=build /convex/target/release/convex-local-backend /usr/local/bin/convex-local-backend

# Expose the required port
EXPOSE 3210
EXPOSE 3211

# Set the binary as executable and run it
CMD ["./convex-local-backend"]
