    一个持久ws连接：
    一次订阅只能订阅一个链上的一种类型数据
    一个ws长连接可以订阅多个链多种类型数据，而一种类型数据可以订阅多次，订阅多次可能会有重复情况，
    因为根据subscriber ID来区分，确实会重复推送
    而取消订阅也可以多次，根据 subscriber ID 来区分

    考虑需要处理的情况：
    首先是需要基本跟substrate返回值要一样
    处理重复订阅问题

    subscriptions:  map (chain_name ++ method ++ client_id) -> (keys set + request id)
