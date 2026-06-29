#!/bin/sh
set -e

echo "Waiting for database to be ready..."
until pg_isready -d "$DATABASE_URL" 2>/dev/null; do
  echo "Database is unavailable - sleeping"
  sleep 1
done

echo "Database is ready!"
echo "Starting OAuth service..."
exec ./oauth
