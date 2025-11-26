#!/bin/bash
set -ex

CLI=./target/debug/worky-cli

echo "Loading worker..."
$CLI load --address 127.0.0.1:3000 --path $(pwd)/worky-api/test/hello.js --name worker1

echo "Waiting for worker to start..."
sleep 2

echo "Testing worker..."
RESPONSE=$(curl -v http://127.0.0.1:3000 2>&1)
echo "Response: $RESPONSE"

if [[ "$RESPONSE" == *"Hello from JS"* ]]; then
  echo "Worker test PASSED"
else
  echo "Worker test FAILED"
  exit 1
fi

echo "Unloading worker..."
$CLI unload --address 127.0.0.1:3000

echo "Testing worker after unload (should fail)..."
if curl -s http://127.0.0.1:3000; then
  echo "Worker still running! FAILED"
  exit 1
else
  echo "Worker unloaded successfully. PASSED"
fi
