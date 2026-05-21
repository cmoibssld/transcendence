#!/bin/bash
set -e

# Wait for database to be ready
echo "Waiting for database to be ready..."
until pg_isready -h db -U diesel -d transcendance 2>/dev/null; do
  echo "Database is unavailable - sleeping"
  sleep 1
done

echo "Database is ready!"

# Run migrations
echo "Running migrations..."
cd /app
diesel migration run --database-url $DATABASE_URL || echo "Migrations already applied or failed"

# Start the OAuth service
echo "Starting OAuth service..."
cd /app
exec ./oauth
