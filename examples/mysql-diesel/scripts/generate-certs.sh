#!/usr/bin/env bash

openssl genrsa 2048 > certs/root-ca-key.pem
openssl req -new -x509 -nodes -days 3600 -subj "/C=GB/ST=London/L=London/CN=mysql-diesel" -key certs/root-ca-key.pem -out certs/root-ca.pem
openssl req -newkey rsa:2048 -days 3600 -nodes -subj "/C=GB/ST=London/L=London/CN=mysql-diesel-server" -keyout certs/server-key.pem -out certs/server-req.pem
openssl rsa -in certs/server-key.pem -out certs/server-key.pem
openssl x509 -req -in certs/server-req.pem -days 3600 -CA certs/root-ca.pem -CAkey certs/root-ca-key.pem -set_serial 01 -out certs/server-cert.pem
openssl verify -CAfile certs/root-ca.pem certs/server-cert.pem
openssl req -newkey rsa:2048 -days 3600 -nodes -subj "/C=GB/ST=London/L=London/CN=mysql-diesel-client" -keyout certs/client-key.pem -out certs/client-req.pem
openssl rsa -in certs/client-key.pem -out certs/client-key.pem
openssl x509 -req -in certs/client-req.pem -days 3600 -CA certs/root-ca.pem -CAkey certs/root-ca-key.pem -set_serial 01 -out certs/client-cert.pem
openssl verify -CAfile certs/root-ca.pem certs/client-cert.pem
