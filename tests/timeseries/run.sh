#!/usr/bin/env bash

BASE=$(dirname "$0")

PENNSIEVE_API_LOC="https://dev.pennsieve.io"
AGENT_DEV_ACCOUNT_EMAIL="agent-test@pennsieve.com"
AGENT_DEV_ACCOUNT_PASSWORD="{Gvn4/D8g_FyxK\N+2"
WAIT_TIME=2

echo "- Building"
cargo build --quiet

echo "- Logging in to the platform"
TOKEN=$(http --body "$PENNSIEVE_API_LOC/account/login" email="$AGENT_DEV_ACCOUNT_EMAIL" password="$AGENT_DEV_ACCOUNT_PASSWORD" | jq -r .sessionToken)

echo "- Got token: $TOKEN"

echo "- Running agent in server mode"
if [ -n "$DEBUG" ]; then
  cargo run --quiet -- server &
else
  cargo run --quiet -- server > /dev/null 2>&1 & # No output
fi
#target/debug/pennsieve server > /dev/null 2>&1 & # No output

echo "- Found agent PID: $AGENT_PID"
AGENT_PID=$!

echo "- Getting the local timeseries proxy port"
LOCAL_PORT=$(cargo run --quiet -- config show timeseries_local_port)

if [ -n "$LOCAL_PORT" ]; then
  echo "- Found port $LOCAL_PORT"
else
  echo "- Couldn't find port!"
  exit 1
fi

echo "- Waiting for ${WAIT_TIME}s"
sleep $WAIT_TIME

echo "- Running the test"
START_TIME=$(date '+%s')
DEBUG=$DEBUG node "$(realpath "$BASE/main.js")" "$LOCAL_PORT" "$TOKEN" "$(realpath "$BASE/request.json")"
TEST_EXIT_CODE=$?
END_TIME=$(date '+%s')

echo "- Test ran for $((END_TIME - START_TIME))s"

echo "- Test exited with code $TEST_EXIT_CODE"

echo "- Sending SIGINT to the agent"
kill -2 $AGENT_PID

echo "- Waiting for agent process $AGENT_PID to exit"
wait $AGENT_PID

if [ $TEST_EXIT_CODE -eq 0 ]; then
  echo "OK"
  exit 0
else
  echo "FAILED"
  exit 1
fi
