## Clash Butler

现在 Clash 配置文件如日中天，各种节点都有 Clash 配置文件格式，不过 Clash
对于用户界面的开发迭代并没有很快。

想之前用得最舒服的一个电脑端的代理软件还得是 [V2rayNG](https://github.com/2dust/v2rayNG)
，支持节点测速，测延迟，删除导出，自动排序等等（指节点管理这一块）。

作为一个「忠实的白嫖节点的人」，Clash 节点不允许做删除和新增，只能添加额外的配置，在大佬发新的节点会导致配置列表就会巨长，管理成本变高。

并且分享的节点基本是日抛类型，很快就会失效，不过一个订阅中个别链接又是可用的，
此时就急需一个工具来测速合并多个配置文件，且为了更好和 Clash 客户端配合，生成的链接需要固定的，似乎没有这方面的工具，不如咱就写一个吧？！

> [!IMPORTANT]
> 作为 Rust 初学者，代码有很多 🐛，目前仅满足基础的自用功能，不过后续会不断完善滴，以下是个人开发和使用环境，如果你碰巧有环境且需要的话可以一试

<p align="center">
  <img alt="vscode" src="https://img.shields.io/badge/Visual%20Studio%20Code-0078d7.svg?style=flat-square&logo=visual-studio-code&logoColor=white" >
  <img alt="Rust" src="https://img.shields.io/badge/Rust 2021-%23000000.svg?style=flat-square&logo=rust&logoColor=white" >
  <img alt="MacOS" src="https://img.shields.io/badge/Sonoma%2014.3.1-000000?style=flat-square&logo=macos&logoColor=F0F0F0" />
</p>

### 使用方式

```shell
cargo run
```

1. 添加新的订阅链接并测试：http://localhost:3000/add?url=sub_url
2. Clash 固定配置文件地址：http://localhost:3000/subs/release/config.yaml
3. 测试当前 test 文件中的延迟：http://localhost:3000/test
4. 合并缓存的所有订阅并延迟：http://localhost:3000/test/all
5. subconverter 服务地址（常驻，与服务端共存亡）：http://localhost:25500/version
6. clash meta 服务地址（测速过程中是开启的，测完速就停止）：http://localhost:9090/version

### 如何订阅转换合并

> 借助 [subconverter](https://github.com/MetaCubeX/subconverter) 进行 Clash 订阅生成和订阅合并，API
> 文档：[点我](https://github.com/tindy2013/subconverter/blob/master/README-cn.md#%E7%AE%80%E6%98%93%E7%94%A8%E6%B3%95)

1. 订阅链接合并

   `http://127.0.0.1:25500/sub?target=clash&url=sub1|sub2|sub3`
2. 订阅链接生成

   `http://127.0.0.1:25500/sub?target=clash&url=sub1&config=external_config_url`
3. 订阅节点去除（测速后重新生成订阅并排除没有速度的节点）

   `http://127.0.0.1:25500/sub?target=clash&url=sub1&exclude=regex1|regex2`

### 如何节点测速

> 借助 [Meta Kernel](https://github.com/MetaCubeX/mihomo/tree/Meta) url-test 代理分组进行测速，API
> 文档：[点我](https://wiki.metacubex.one/api/)

1. 获取节点分组延迟信息

   `http://127.0.0.1:9090/providers/proxies/自动选择`

### 如何针对指定的 URL 测速并分组？

个别网站对节点要求极高，可能当前能用的节点都不支持到实际用到的时候不可能一个一个测速，这个不实际且花费时间较长，clash meta
内核目前似乎有指定 testUrl 单独测速的功能但是还不知道怎么用，后续研究明白再加入