import socket


server_ip = '0.0.0.0'
server_port = 5000

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.connect((server_ip, server_port))
print(sock.recv(1024))
