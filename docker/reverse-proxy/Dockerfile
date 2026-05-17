FROM alpine:3.21.5

RUN <<EOF
apt update
apk add --no-cache \
	nginx
EOF

COPY conf/nginx.conf /etc/nginx/nginx.conf

EXPOSE 8080

CMD ["nginx"]
