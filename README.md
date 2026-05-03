# rust_ping

Rust で作った最小構成の `ping` コマンドです。依存クレートを使わず、IPv4 の ICMP Echo Request/Reply を直接扱います。

## 使い方

```bash
cargo run -- --help
sudo cargo run -- 8.8.8.8
sudo cargo run -- example.com -c 5 -W 1500 -i 500
sudo cargo run --bin ping_reply
```

## できること

- ホスト名または IPv4 アドレスを解決
- ICMP Echo Request を送信
- Echo Reply を受信して RTT を表示
- 送受信数と min/avg/max の統計を表示
- Echo Request を受信して Echo Reply を返す responder を起動

## 注意点

- 現状は IPv4 専用です
- Linux / macOS 向けです
- Raw socket を使うため、通常は `sudo` か root 権限が必要です
