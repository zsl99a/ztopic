#!/bin/bash

rm -f ca.key ca.crt ca.srl server.key server.csr server.crt client.key client.csr client.crt

# Generate CA
openssl req -newkey rsa:4096 -x509 -nodes -keyout ca.key -out ca.crt -days 3650 -subj "/CN=Root CA"

# Generate server certificates
openssl req -new -newkey rsa:2048 -nodes -keyout server.key -out server.csr -subj "/CN=localhost"
openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out server.crt -days 365 -sha384 \
        -extensions req_ext -extfile <(printf "[req_ext]\nsubjectAltName=DNS:localhost")

# Generate client certificates
openssl req -new -newkey rsa:2048 -nodes -keyout client.key -out client.csr -subj "/CN=localhost"
openssl x509 -req -in client.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out client.crt -days 365 -sha384 \
        -extensions req_ext -extfile <(printf "[req_ext]\nsubjectAltName=DNS:localhost")
