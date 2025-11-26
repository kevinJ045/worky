#!/bin/bash
# Start daemon
cargo run -p worky-cli -- daemon > daemon.log 2>&1 &
DAEMON_PID=$!
echo "Daemon started with PID $DAEMON_PID"

sleep 10

# Register worker
echo "Registering worker..."
cargo run -p worky-cli -- load --address "127.0.0.1:3001" --path "worky-api/test/hello.js" --name "worker1"

sleep 5

# Check if listening
echo "Checking if listening on 3001..."
if curl -s http://127.0.0.1:3001; then
  echo "Success! Listening."
else
  echo "Failed to connect."
fi

# Register again
echo "Registering worker again..."
cargo run -p worky-cli -- load --address "127.0.0.1:3001" --path "worky-api/test/hello.js" --name "worker1"

sleep 5

# Check if listening
echo "Checking if listening on 3001..."
if curl -s http://127.0.0.1:3001; then
  echo "Success! Listening."
else
  echo "Failed to connect."
fi

kill $DAEMON_PID
cat daemon.log
