###### FRONTEND ######

# Build frontend resources in node container
FROM node:22-slim AS frontend-build

# Compile-time feature flag: enable the igcviewer "view track" button
ARG FEATURE_IGCVIEWER=0
ENV VITE_FEATURE_IGCVIEWER=${FEATURE_IGCVIEWER}

# Build with npm
COPY frontend /build/flugbuech/
RUN cd /build/flugbuech && npm install && npm run build


###### BACKEND ######

# Build server resources in rust container (Debian 12: buster is EOL)
FROM docker.io/rust:1-bookworm AS backend-build

# Compile-time feature flag: enable the igcviewer integration (cargo feature
# "igcviewer")
ARG FEATURE_IGCVIEWER=0

# Build binary
COPY . /build/flugbuech/
RUN cd /build/flugbuech \
 && cargo build --release $(if [ "${FEATURE_IGCVIEWER}" = "1" ]; then printf -- "--features igcviewer"; fi)


###### RUNTIME ######

FROM debian:13-slim

# Install init process and other dependencies; upgrade pulls security
# fixes (e.g. libpq5/libc) that the base tag may predate.
RUN apt-get update -q \
 && apt-get -y -q upgrade \
 && apt-get -y -q install --no-install-recommends dumb-init libpq5 \
 && rm -rf /var/lib/apt/lists/*

# Create users and directories
RUN groupadd -g 1000 flugbuech \
 && useradd -m -u 1000 -g 1000 -s /bin/bash flugbuech
RUN mkdir /flugbuech \
 && chown flugbuech:flugbuech /flugbuech/ \
 && chmod 0700 /flugbuech/
RUN mkdir /static \
 && chown flugbuech:flugbuech /static/ \
 && chmod 0700 /static/

# Copy backend files
COPY --from=backend-build --chown=flugbuech:flugbuech /build/flugbuech/target/release/flugbuech-api /flugbuech/flugbuech-api
COPY --from=backend-build --chown=flugbuech:flugbuech /build/flugbuech/Rocket.toml /flugbuech/Rocket.toml

# Copy frontend files
COPY --from=frontend-build --chown=flugbuech:flugbuech /build/flugbuech/build /flugbuech/static

# Specify workdir and user
WORKDIR /flugbuech
USER flugbuech

# Create volumes
VOLUME ["/static"]

ENV RUST_BACKTRACE=1
ADD entrypoint.sh /flugbuech/entrypoint.sh
ENTRYPOINT ["/usr/bin/dumb-init", "--"]
CMD ["/bin/bash", "/flugbuech/entrypoint.sh"]
