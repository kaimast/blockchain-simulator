#! /bin/bash

blockchain-sim-test-server &

sleep 0.5

blockchain-sim-test-client send_transactions

sleep 2

blockchain-sim-test-client count_transactions
result=$?

killall blockchain-sim-test-server

exit $result
