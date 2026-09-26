#!/usr/bin/env python3
import os
import socket
import sys

socket_path = sys.argv[1]
mode = int(sys.argv[2], 8)
try:
    os.unlink(socket_path)
except FileNotFoundError:
    pass

server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
server.bind(socket_path)
os.chmod(socket_path, mode)
server.listen()
while True:
    connection, _ = server.accept()
    with connection:
        connection.recv(4096)
        connection.sendall(b"HTTP/1.1 200 OK\r\nContent-Length: 18\r\n\r\nmaos-egress-probe\n")
