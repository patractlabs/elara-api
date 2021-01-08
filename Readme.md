# Elara-api 配置文件说明

配置文件使用 toml 格式

## 依赖的统计服务配置

```toml
[stat]
name = "stat" --统计服务名
url = "localhost:1234" --统计服务器 url
```

## websocket 服务配置

```toml
[ws]
name = "websocket"
url = "localhost:1234" --websocket server url
```

## 链节点配置

```toml
[[chains]]
name = "Polkadot" --链名称
rpcUrl = "localhost:1234" --链 rpc url
wsUrl = "localhost:1234" --链 websocket url
```

## rocket http server 配置

本服务使用 rocket 搭建 http 服务，rocket 配置通过 Rocket.toml 文件设置，参考：  
<https://docs.rs/rocket/0.4.6/rocket/config/index.html>

# 运行

## help

```sh
/path_to_bin/elara-api -h
elara api 0.0.1
Patract Lab
commandline argument parsing

USAGE:
 elara-api [OPTIONS]

FLAGS:
 -h, --help Prints help information
 -V, --version Prints version information

OPTIONS:
 -f, --file <file> A config absolute file path
```

## 启动服务

```sh
/path_to_bin/elara-api -f path_to_config/config.toml
```

note:  
elara-api 可执行文件同级目录下需要存放 Rocket.toml 文件
