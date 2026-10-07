/* [07AA-14] Sidecars SSH/SFTP Coolify (split sin cambios desde coolify.rs). */

/* [114A-3] SSH container con wp-cli vía dockerfile_inline + hardening sshd.
 * - PHP + wp-cli instalados para gestión WordPress vía shell.
 * - backend_net añadida para que wp-cli pueda conectar a MariaDB.
 * - sshd hardening: AllowTcpForwarding=no, X11Forwarding=no, PermitTunnel=no, GatewayPorts=no
 *   aplicados idempotentemente vía custom-cont-init.d script.
 * - Límites de CPU/RAM dinámicos desde plan config. */
pub(super) fn build_compose_ssh(
    sftp_user: &str,
    sftp_password: &str,
    sftp_port: i32,
    ssh_cpu: &str,
    ssh_mem: &str,
) -> String {
    format!(
        r#"  ssh:
        build:
            dockerfile_inline: |
                FROM lscr.io/linuxserver/openssh-server:version-9.9_p2-r0
                RUN apk add --no-cache php83-cli php83-phar php83-json php83-mbstring php83-curl php83-mysqli php83-xml php83-tokenizer bash coreutils \
                        && ln -sf /usr/bin/php83 /usr/bin/php \
                        && curl -sSL https://raw.githubusercontent.com/wp-cli/builds/gh-pages/phar/wp-cli.phar -o /usr/local/bin/wp \
                        && chmod +x /usr/local/bin/wp
                RUN mkdir -p /custom-cont-init.d && \
                        echo '#!/bin/bash' > /custom-cont-init.d/10-harden-ssh && \
                        echo 'grep -q "AllowTcpForwarding no" /config/sshd/sshd_config 2>/dev/null || {{' >> /custom-cont-init.d/10-harden-ssh && \
                        echo '  echo "AllowTcpForwarding no" >> /config/sshd/sshd_config' >> /custom-cont-init.d/10-harden-ssh && \
                        echo '  echo "X11Forwarding no" >> /config/sshd/sshd_config' >> /custom-cont-init.d/10-harden-ssh && \
                        echo '  echo "PermitTunnel no" >> /config/sshd/sshd_config' >> /custom-cont-init.d/10-harden-ssh && \
                        echo '  echo "GatewayPorts no" >> /config/sshd/sshd_config' >> /custom-cont-init.d/10-harden-ssh && \
                        echo '}}' >> /custom-cont-init.d/10-harden-ssh && \
                        chmod +x /custom-cont-init.d/10-harden-ssh
        environment:
            - PUID=33
            - PGID=33
            - TZ=UTC
            - USER_NAME={sftp_user}
            - USER_PASSWORD={sftp_password}
            - PASSWORD_ACCESS=true
            - SUDO_ACCESS=false
            - LOG_STDOUT=true
        volumes:
            - 'wordpress-data:/home/{sftp_user}/html'
        ports:
            - '{sftp_port}:2222'
        restart: unless-stopped
        networks:
            - ssh_net
            - backend_net
        cap_drop:
            - ALL
        cap_add:
            - CHOWN
            - SETUID
            - SETGID
            - DAC_OVERRIDE
            - NET_BIND_SERVICE
        security_opt:
            - no-new-privileges:true
        deploy:
            resources:
                limits:
                    cpus: '{ssh_cpu}'
                    memory: {ssh_mem}
                reservations:
                    memory: 64M
"#
    )
}

/* [155A-13] SSH/SFTP para hosting normal sin PHP/WP-CLI ni red backend. */
pub(super) fn build_compose_static_ssh(
    sftp_user: &str,
    sftp_password: &str,
    sftp_port: i32,
    ssh_cpu: &str,
    ssh_mem: &str,
) -> String {
    format!(
        r#"  ssh:
        build:
            dockerfile_inline: |
                FROM lscr.io/linuxserver/openssh-server:version-9.9_p2-r0
                RUN apk add --no-cache bash coreutils
                RUN mkdir -p /custom-cont-init.d && \
                        echo '#!/bin/bash' > /custom-cont-init.d/10-harden-ssh && \
                        echo 'grep -q "AllowTcpForwarding no" /config/sshd/sshd_config 2>/dev/null || {{' >> /custom-cont-init.d/10-harden-ssh && \
                        echo '  echo "AllowTcpForwarding no" >> /config/sshd/sshd_config' >> /custom-cont-init.d/10-harden-ssh && \
                        echo '  echo "X11Forwarding no" >> /config/sshd/sshd_config' >> /custom-cont-init.d/10-harden-ssh && \
                        echo '  echo "PermitTunnel no" >> /config/sshd/sshd_config' >> /custom-cont-init.d/10-harden-ssh && \
                        echo '  echo "GatewayPorts no" >> /config/sshd/sshd_config' >> /custom-cont-init.d/10-harden-ssh && \
                        echo '}}' >> /custom-cont-init.d/10-harden-ssh && \
                        chmod +x /custom-cont-init.d/10-harden-ssh
        environment:
            - PUID=101
            - PGID=101
            - TZ=UTC
            - USER_NAME={sftp_user}
            - USER_PASSWORD={sftp_password}
            - PASSWORD_ACCESS=true
            - SUDO_ACCESS=false
            - LOG_STDOUT=true
        volumes:
            - 'site-data:/home/{sftp_user}/html'
        ports:
            - '{sftp_port}:2222'
        restart: unless-stopped
        networks:
            - ssh_net
        cap_drop:
            - ALL
        cap_add:
            - CHOWN
            - SETUID
            - SETGID
            - DAC_OVERRIDE
            - NET_BIND_SERVICE
        security_opt:
            - no-new-privileges:true
        deploy:
            resources:
                limits:
                    cpus: '{ssh_cpu}'
                    memory: {ssh_mem}
                reservations:
                    memory: 64M
"#
    )
}
