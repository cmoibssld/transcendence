FROM node:20-alpine

WORKDIR /app

COPY game.json .
COPY blackjack.js .

# Non root user to make it safer
RUN addgroup -g 1001 -S nodejs && \
	adduser -S nextjs -u 1001
USER nextjs

ENV GAME_IO_FILE=/app/game.json

CMD ["node", "blackjack.js"]

