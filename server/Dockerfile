# Stage 1: Build the backend
FROM rustlang/rust:nightly AS builder

# Set working directory
WORKDIR /app

# Copy all projects
COPY . .

# Build the backend
RUN cargo build --release -p server

# Stage 2: Create the final image
FROM debian:stable

# Set working directory
WORKDIR /app

# Copy the backend binary
COPY --from=builder /app/target/release/server /app/server

# Expose the port the server runs on
ARG PORT=3001
EXPOSE ${PORT}

# Set the command to run the server
CMD ["/app/server"]
