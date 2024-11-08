# docker container Network fix

as it is named it helps with the vpn container network issue when updating/recreating the VPN container  
for example 
```yml
services:
    gluetun:
        cap_add:
        - NET_ADMIN
        devices:
        - /dev/net/tun:/dev/net/tun
        environment:
            # See https://github.com/qdm12/gluetun-wiki/tree/main/setup#setup
        image: qmcgaw/gluetun
        ports:
            - 8080:8080
    qbittorrent:
        environment:
            - WEBUI_PORT=8080
        image: lscr.io/linuxserver/qbittorrent:latest
        network_mode: container:gluetun
        volumes:
            - /opt/stacks/qbittorrent:/config
            - ./downloads:/downloads
        depends_on:
            gluetun
    watchtower:
        image: containrrr/watchtower
        volumes:
            - /var/run/docker.sock:/var/run/docker.sock
            - /opt/stacks/watchtower/config.json:/config.json
```

when watchtower automatically updates the gluetun container the qbittorrent container will loose network connection since it is still linked to the old gluetun container ID. Simply restarting the qbittorrent container wont fix this, it needs to be recreated from the compose. (this isnt actually an issue with watchtower as of [1429](https://github.com/containrrr/watchtower/pull/1429), bit is still an issue for manual recreating)

what this does is when it finds a container that has a `networkHost:"container:{containerID}"` that doesn't exist anymore it recreates the container via the compose file that the container is apart of
`docker compose -f {compose_yml} up -d --remove-orphans --force-recreate {service_name}`

just running 
```yml
services:
    dockercontainernetworkfix:
        image: ghcr.io/robonau/docker_container_network_fix
        restart: unless-stopped
        volumes:
            - /var/run/docker.sock:/var/run/docker.sock

            # If you want to use private registries, you need to share the auth file with 
            # - /root/.docker/:/root/.docker

            # Stacks Directory
            # ⚠️ READ IT CAREFULLY. If you did it wrong, it wont work.
            # ⚠️ 1. FULL path only. No relative path (MUST)
            # ⚠️ 2. Left Stacks Path === Right Stacks Path (MUST)
            - /opt/stacks:/opt/stacks:ro
        environment:
            # Tell docker container network fix where is your stacks directory
            - COMPOSE_STACKS_DIR=/opt/stacks
```
will fix the issue

## shoutouts:
[dockge](https://github.com/louislam/dockge)  
I recently switched from k3s to docker + dockge (truenas scale) on my home server and ran in to this issue again so i decided to fix it this time.

## caveats
this probably wont work with portainer or most container management systems (other than dockge) since it relies heavily on the compose files existing
