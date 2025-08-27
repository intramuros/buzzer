# Stage 1: Build the frontend
FROM rustlang/rust:nightly AS builder

# Install wasm target
RUN rustup target add wasm32-unknown-unknown

# Install dioxus-cli
SHELL [ "/bin/bash", "-c" ]
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall dioxus-cli

# Set working directory
WORKDIR /app

# Copy the entire workspace
COPY . .

# Set working directory to frontend
WORKDIR /app/frontend

# Build the frontend
RUN dx bundle --out-dir dist --platform web

# Stage 2: Create the final image
FROM nginx:stable-alpine

# Copy the frontend assets
COPY --from=builder /app/frontend/dist/public /usr/share/nginx/html

# Copy the nginx configuration
COPY frontend/nginx.conf /etc/nginx/conf.d/default.conf

# Expose the port nginx runs on
EXPOSE 80

# Set the command to run nginx
CMD ["nginx", "-g", "daemon off;"]
