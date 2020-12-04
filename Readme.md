# 配置文件说明：  
配置文件使用toml格式  
## 依赖的统计服务配置
[stat]  
name = "stat"  --统计服务名  
url = "localhost:1234"  --统计服务器url 
## websocket服务配置  
[ws]  
name = "websocket"  
url = "localhost:1234"   --websocket server url 
## 链节点配置  
[[chains]]  
name = "Polkadot" --链名称  
rpcUrl = "localhost:1234" --链rpc url  
wsUrl = "localhost:1234" --链websocket url  
## rocket http server配置
本服务使用rocket搭建http服务，rocket配置通过Rocket.toml文件设置，参考：  
https://docs.rs/rocket/0.4.6/rocket/config/index.html  

# 运行：  
## help  
/path_to_bin/elara-api -h  
elara api 0.0.1  
Patract Lab  
commandline argument parsing  

USAGE:  
    elara-api [OPTIONS]  

FLAGS:  
    -h, --help       Prints help information  
    -V, --version    Prints version information  

OPTIONS:  
    -f, --file <file>    A config absolute file path  

## 启动服务  
/path_to_bin/elara-api -f path_to_config/config.toml  
note:  
elara-api可执行文件同级目录下需要存放Rocket.toml文件