Simple p2p gossiping app. It has the following command line arguments
- --bind - bind address for new connections
- --ping-period-ms - ping others peers period in milliseconds
- --connect (optional) - initial remote peer

# Example:
Run three peers

```cargo run -- --bind 127.0.0.1:4555 --ping-period-ms 5000```

```cargo run -- --bind 127.0.0.1:4556 --ping-period-ms 1000 --connect 127.0.0.1:4555```

```cargo run -- --bind 127.0.0.1:4557 --ping-period-ms 1000 --connect 127.0.0.1:4556```

Example output from the third peer:

```
[2021-04-19T22:31:00Z INFO  p2p_test_task::listener::runner] Peer '127.0.0.1:4556' connected
[2021-04-19T22:31:00Z INFO  p2p_test_task::listener::runner] Asking peers from 127.0.0.1:4556
[2021-04-19T22:31:00Z INFO  p2p_test_task] Got ping 'Random message' from 127.0.0.1:4556
[2021-04-19T22:31:00Z INFO  p2p_test_task::listener::runner] Peer '127.0.0.1:4555' connected
[2021-04-19T22:31:00Z INFO  p2p_test_task] Got ping 'Random message' from 127.0.0.1:4555
[2021-04-19T22:31:01Z INFO  p2p_test_task] Got ping 'Random message' from 127.0.0.1:4556
[2021-04-19T22:31:02Z INFO  p2p_test_task] Got ping 'Random message' from 127.0.0.1:4556
[2021-04-19T22:31:03Z INFO  p2p_test_task] Got ping 'Random message' from 127.0.0.1:4556
[2021-04-19T22:31:04Z INFO  p2p_test_task] Got ping 'Random message' from 127.0.0.1:4556
[2021-04-19T22:31:05Z INFO  p2p_test_task] Got ping 'Random message' from 127.0.0.1:4556
[2021-04-19T22:31:05Z INFO  p2p_test_task] Got ping 'Random message' from 127.0.0.1:4555
[2021-04-19T22:31:06Z INFO  p2p_test_task] Got ping 'Random message' from 127.0.0.1:4556
[2021-04-19T22:31:07Z INFO  p2p_test_task] Got ping 'Random message' from 127.0.0.1:4556
[2021-04-19T22:31:08Z INFO  p2p_test_task] Got ping 'Random message' from 127.0.0.1:4556
[2021-04-19T22:31:09Z INFO  p2p_test_task] Got ping 'Random message' from 127.0.0.1:4556
[2021-04-19T22:31:10Z INFO  p2p_test_task::listener::runner] Asking peers from 127.0.0.1:4555
```