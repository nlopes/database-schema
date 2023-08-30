ARG VERSION
FROM postgres:${VERSION}-alpine

COPY ./certs /var/lib/postgresql/certs

# Fix permissions so that server starts
RUN chown 70:70 /var/lib/postgresql/certs/server-cert.pem /var/lib/postgresql/certs/server-key.pem
RUN chmod 0600 /var/lib/postgresql/certs/server-cert.pem /var/lib/postgresql/certs/server-key.pem
