# SIP 报文与头字段说明

## 1. 文档目的

本文介绍 SIP（Session Initiation Protocol，会话初始协议）的报文结构、请求方法、响应状态码和常见 Header 字段，并结合当前项目说明：

- SIP Request 和 Response 的结构；
- INVITE、REGISTER、ACK、BYE、CANCEL 等方法；
- Via、From、To、Call-ID、CSeq 等核心 Header；
- SIP Transaction 与 Dialog 如何标识；
- Contact、Route、Record-Route 的作用；
- Digest 认证相关 Header；
- SIP 如何通过 `Content-Type: application/sdp` 携带 SDP；
- 当前项目的 LAN 直拨和 Cloud Registrar 流程；
- 如何通过抓包排查 SIP 问题。

说明：SIP Header 是可扩展的。本文覆盖 RFC 3261 核心 Header、常见扩展 Header 及当前项目会遇到的字段，但无法枚举所有厂商私有 `P-` Header 和历史扩展。

---

## 2. SIP 报文总体结构

SIP 是文本协议，报文由以下部分组成：

```text
SIP Message
 |
 +-- Start-Line
 |    |
 |    +-- Request-Line：请求
 |    |
 |    +-- Status-Line：响应
 |
 +-- Header Fields
 |
 +-- 空行 CRLF
 |
 +-- Message Body（可选）
      |
      +-- SDP
      +-- XML
      +-- 文本
      +-- 其他 MIME Body
```

网络封装：

```text
Ethernet
└── IP
    └── UDP / TCP / TLS
        └── SIP
            ├── Start-Line
            ├── Headers
            ├── 空行
            └── Body（可选）
```

当前项目主要使用：

```text
UDP
└── SIP
```

本地 SIP 监听地址：

```text
0.0.0.0:5060
```

---

## 3. SIP 请求报文结构

请求报文第一行是 Request-Line：

```text
<Method> <Request-URI> SIP/2.0
```

示例：

```text
INVITE sip:1008@192.168.1.20 SIP/2.0
Via: SIP/2.0/UDP 192.168.1.10:5060;branch=z9hG4bK-001;rport
Max-Forwards: 70
From: <sip:1007@192.168.1.10>;tag=from-tag-001
To: <sip:1008@192.168.1.20>
Call-ID: call-001@192.168.1.10
CSeq: 1 INVITE
Contact: <sip:1007@192.168.1.10:5060;transport=udp>
Allow: INVITE, ACK, CANCEL, BYE, OPTIONS
Supported: replaces, timer
Content-Type: application/sdp
Content-Length: <SDP长度>

v=0
o=- 10001 10001 IN IP4 192.168.1.10
s=-
c=IN IP4 192.168.1.10
t=0 0
m=audio 10000 RTP/AVP 0 8
a=sendrecv
```

其中：

```text
INVITE ... SIP/2.0       Request-Line
Via ...                  Header
From ...                 Header
...                      Header
Content-Length ...       Header
                         空行
v=0 ...                  SDP Body
```

---

## 4. SIP 响应报文结构

响应报文第一行是 Status-Line：

```text
SIP/2.0 <Status-Code> <Reason-Phrase>
```

示例：

```text
SIP/2.0 200 OK
Via: SIP/2.0/UDP 192.168.1.10:5060;branch=z9hG4bK-001;rport=5060
From: <sip:1007@192.168.1.10>;tag=from-tag-001
To: <sip:1008@192.168.1.20>;tag=to-tag-001
Call-ID: call-001@192.168.1.10
CSeq: 1 INVITE
Contact: <sip:1008@192.168.1.20:5060;transport=udp>
Content-Type: application/sdp
Content-Length: <SDP长度>

v=0
o=- 20001 20001 IN IP4 192.168.1.20
s=-
c=IN IP4 192.168.1.20
t=0 0
m=audio 40000 RTP/AVP 8
a=sendrecv
```

---

## 5. Request-Line

格式：

```text
<Method> <Request-URI> SIP/2.0
```

示例：

```text
INVITE sip:1008@192.168.1.20 SIP/2.0
```

拆解：

| 字段 | 示例 | 含义 |
| --- | --- | --- |
| Method | `INVITE` | 请求方法 |
| Request-URI | `sip:1008@192.168.1.20` | 当前请求目标 |
| SIP Version | `SIP/2.0` | SIP 协议版本 |

Request-URI 是本次请求的路由目标，不一定与 `To`、`Contact` 或最终媒体地址相同。

---

## 6. Status-Line

格式：

```text
SIP/2.0 <Status-Code> <Reason-Phrase>
```

示例：

```text
SIP/2.0 180 Ringing
SIP/2.0 200 OK
SIP/2.0 401 Unauthorized
SIP/2.0 486 Busy Here
```

`Reason-Phrase` 主要用于人类阅读，程序应以数字状态码为准。

---

## 7. SIP URI

典型 SIP URI：

```text
sip:1008@192.168.1.20
sip:1008@example.com
sip:1008@192.168.1.20:5060;transport=udp
```

格式：

```text
sip:<user>@<host>:<port>;<uri-parameters>?<headers>
```

示例拆解：

```text
sip:1008@192.168.1.20:5060;transport=udp
    |    |              |    |
    user host           port URI parameter
```

常见 URI 参数：

| 参数 | 含义 |
| --- | --- |
| `transport=udp` | 使用 UDP |
| `transport=tcp` | 使用 TCP |
| `transport=tls` | 使用 TLS |
| `user=phone` | user 部分按电话号码解释 |
| `lr` | Loose Routing，常见于 Route/Record-Route |
| `maddr` | 指定服务器地址 |

当前项目账号 AOR 示例：

```text
<sip:<username>@<host>;transport=udp>
```

LAN 账号额外使用 Baresip 账号参数：

```text
<sip:<username>@<host>;transport=udp>;regint=0
```

注意：`regint=0` 位于 `>` 外，是 Baresip 账号参数，不是 SIP URI 参数。

---

## 8. 核心 Header 总览

常见 SIP 请求至少需要关注：

```text
Via
Max-Forwards
From
To
Call-ID
CSeq
Contact（建立 Dialog、REGISTER 等场景）
Content-Length
```

携带 SDP 时还需要：

```text
Content-Type: application/sdp
Content-Length: <长度>
```

一组典型核心 Header：

```text
Via: SIP/2.0/UDP 192.168.1.10:5060;branch=z9hG4bK-001;rport
Max-Forwards: 70
From: <sip:1007@example.com>;tag=from-tag-001
To: <sip:1008@example.com>
Call-ID: call-001@192.168.1.10
CSeq: 1 INVITE
Contact: <sip:1007@192.168.1.10:5060;transport=udp>
Content-Type: application/sdp
Content-Length: 200
```

---

## 9. `Via`

示例：

```text
Via: SIP/2.0/UDP 192.168.1.10:5060;branch=z9hG4bK-001;rport
```

`Via` 描述 SIP 请求经过的传输节点，并用于响应原路返回。

格式：

```text
Via: SIP/2.0/<transport> <sent-by>;<parameters>
```

主要字段：

| 字段 | 示例 | 含义 |
| --- | --- | --- |
| transport | `UDP` | SIP 传输协议 |
| sent-by | `192.168.1.10:5060` | 发送方声明的地址 |
| branch | `z9hG4bK-001` | SIP Transaction 分支标识 |
| rport | 无值或端口 | 请求服务器按报文实际源端口返回响应 |
| received | `192.168.1.10` | 接收方观察到的实际源 IP |

### 9.1 `branch`

```text
branch=z9hG4bK-001
```

RFC 3261 Transaction branch 使用魔术前缀：

```text
z9hG4bK
```

不同 Transaction 通常使用不同 branch。CANCEL 与被取消的 INVITE 在事务匹配上有特殊关联要求。

### 9.2 `rport`

请求：

```text
Via: SIP/2.0/UDP 192.168.1.10:5060;branch=z9hG4bK-001;rport
```

响应可能变为：

```text
Via: SIP/2.0/UDP 192.168.1.10:5060;
     branch=z9hG4bK-001;
     rport=42000;
     received=203.0.113.10
```

表示服务器实际看到报文来自：

```text
203.0.113.10:42000
```

`rport` 常用于 UDP NAT 场景。

### 9.3 多个 Via

经过 Proxy 时，每个 Proxy 在最上方添加自己的 Via：

```text
Via: SIP/2.0/UDP proxy2.example.com;branch=z9hG4bK-003
Via: SIP/2.0/UDP proxy1.example.com;branch=z9hG4bK-002
Via: SIP/2.0/UDP client.example.com;branch=z9hG4bK-001
```

响应按照 Via 从上到下逐跳返回。

---

## 10. `Max-Forwards`

```text
Max-Forwards: 70
```

作用类似 IP TTL，防止 SIP 请求在 Proxy 之间无限循环。

每经过一个 SIP Proxy，数值减 1：

```text
70 -> 69 -> 68 -> ...
```

减到 0 后仍未到达目标，服务器通常返回：

```text
483 Too Many Hops
```

---

## 11. `From`

```text
From: "Door 1007" <sip:1007@example.com>;tag=from-tag-001
```

表示请求的逻辑发起方。

组成：

```text
显示名：Door 1007
URI：sip:1007@example.com
tag：from-tag-001
```

`From` 不一定等于网络报文真实源 IP，也不一定等于 `Contact`。

### 11.1 From Tag

```text
tag=from-tag-001
```

Tag 用于区分 Dialog 端点。初始请求的发起方生成 From Tag。

---

## 12. `To`

初始 INVITE：

```text
To: <sip:1008@example.com>
```

临时或最终响应：

```text
To: <sip:1008@example.com>;tag=to-tag-001
```

`To` 表示逻辑接收方。

被叫方接受或建立对话时生成 To Tag。Forking 场景下，不同被叫分支可以产生不同 To Tag，从而形成不同早期 Dialog。

---

## 13. `Call-ID`

```text
Call-ID: call-001@192.168.1.10
```

`Call-ID` 是一次 SIP 呼叫或注册序列的全局标识之一。

同一个 Dialog 内的 INVITE、ACK、BYE 等请求保持相同 Call-ID：

```text
INVITE  Call-ID: call-001
200 OK  Call-ID: call-001
ACK     Call-ID: call-001
BYE     Call-ID: call-001
200 OK  Call-ID: call-001
```

Call-ID 本身不够唯一标识 Dialog，还需要双方 Tag。

---

## 14. SIP Dialog 标识

一个 SIP Dialog 由以下三部分标识：

```text
Call-ID
Local Tag
Remote Tag
```

从主叫视角：

```text
Dialog ID = Call-ID + From Tag + To Tag
```

例如：

```text
Call-ID: call-001
From tag: from-tag-001
To tag: to-tag-001
```

Dialog 内请求，例如 ACK、re-INVITE、UPDATE、BYE，会沿用该 Dialog 标识。

---

## 15. `CSeq`

```text
CSeq: 1 INVITE
```

格式：

```text
CSeq: <sequence-number> <method>
```

作用：

- 标识请求顺序；
- 区分同一 Dialog 中的不同请求；
- 响应中的 CSeq 必须与对应请求一致。

示例：

```text
INVITE：CSeq: 1 INVITE
ACK：   CSeq: 1 ACK
BYE：   CSeq: 2 BYE
```

ACK 对 2xx INVITE 响应时，数字通常与 INVITE 相同，但 Method 改为 ACK。

CANCEL 通常使用与被取消 INVITE 相同的 CSeq 数字，但 Method 为 CANCEL：

```text
CSeq: 1 INVITE
CSeq: 1 CANCEL
```

---

## 16. `Contact`

```text
Contact: <sip:1007@192.168.1.10:5060;transport=udp>
```

`Contact` 表示发送方希望接收后续 SIP 请求的实际地址。

区别：

```text
From：逻辑身份
Contact：后续 SIP 请求地址
```

例如：

```text
From:    <sip:1007@example.com>
Contact: <sip:1007@192.168.1.10:5060;transport=udp>
```

后续 BYE、re-INVITE 等 Dialog 内请求通常根据对端 Contact 和 Route Set 构造目标。

### 16.1 REGISTER 中的 Contact

```text
REGISTER sip:example.com SIP/2.0
Contact: <sip:1007@192.168.1.10:5060;transport=udp>
Expires: 30
```

含义：

```text
发给 sip:1007@example.com 的来电
应转发到 192.168.1.10:5060
绑定有效期为 30 秒
```

### 16.2 Contact 与 SDP 地址不同

```text
Contact：SIP 后续请求地址
SDP c=：RTP/RTCP 媒体地址
```

不要使用 Contact 推导 RTP 目的地址。

---

## 17. `Route`

```text
Route: <sip:proxy.example.com;lr>
```

`Route` 指定当前请求应经过的 SIP 路由节点。

多个 Route 的顺序有意义：

```text
Route: <sip:proxy1.example.com;lr>
Route: <sip:proxy2.example.com;lr>
```

Dialog 内请求根据建立 Dialog 时得到的 Route Set 发送。

---

## 18. `Record-Route`

```text
Record-Route: <sip:proxy.example.com;lr>
```

Proxy 使用 `Record-Route` 要求自己保留在后续 Dialog 路由中。

初始 INVITE：

```text
主叫 -> Proxy -> 被叫
              |
              +-- Proxy 添加 Record-Route
```

后续 BYE：

```text
主叫 -> 同一个 Proxy -> 被叫
```

区别：

```text
Record-Route：建立 Dialog 时记录路由
Route：后续请求实际使用的路由集合
```

---

## 19. `Content-Type`

```text
Content-Type: application/sdp
```

表示 SIP Body 的 MIME 类型。

常见类型：

```text
application/sdp
application/pidf+xml
application/simple-message-summary
text/plain
multipart/mixed
```

当前项目通话协商主要使用：

```text
application/sdp
```

---

## 20. `Content-Length`

```text
Content-Length: 200
```

表示 Message Body 的字节长度，不包含 Header 和分隔空行。

无 Body 时：

```text
Content-Length: 0
```

SIP over TCP 时必须依靠 `Content-Length` 判断消息边界；UDP 中也应正确设置。

---

## 21. `Content-Disposition`

```text
Content-Disposition: session
```

说明 Body 的处理方式。SDP 常作为 `session` 内容。

也可能带参数：

```text
Content-Disposition: session;handling=required
```

表示接收方必须理解该 Body，否则应拒绝请求。

---

## 22. `Content-Encoding`

```text
Content-Encoding: gzip
```

表示消息体编码方式。普通 SIP SDP 中很少使用。

---

## 23. `Allow`

```text
Allow: INVITE, ACK, CANCEL, OPTIONS, BYE, REFER, NOTIFY, MESSAGE, INFO
```

表示 User Agent 支持的方法。

它用于能力声明，不代表当前请求一定允许执行其中每个操作。

---

## 24. `Supported`

```text
Supported: replaces, timer, 100rel
```

表示发送方支持的 SIP 扩展。

常见 option-tag：

| option-tag | 含义 |
| --- | --- |
| `100rel` | 可靠临时响应/PRACK |
| `timer` | Session Timer |
| `replaces` | Replaces Header |
| `outbound` | SIP Outbound |
| `path` | REGISTER Path 支持 |

支持但不强制对端使用。

---

## 25. `Require`

```text
Require: 100rel
```

表示发送方要求对端必须支持某扩展。

如果对端不支持，应返回：

```text
420 Bad Extension
Unsupported: 100rel
```

区别：

```text
Supported：我支持，但不强制
Require：你必须支持
```

---

## 26. `Proxy-Require`

```text
Proxy-Require: <option-tag>
```

要求路径中的 Proxy 支持指定扩展。现代 SIP 中使用较少。

---

## 27. `Unsupported`

```text
Unsupported: 100rel
```

表示接收方不支持请求中 `Require` 指定的扩展。

---

## 28. `Accept`

```text
Accept: application/sdp, application/pidf+xml
```

表示可接受的消息体类型。

相关 Header：

```text
Accept
Accept-Encoding
Accept-Language
```

---

## 29. `User-Agent`

```text
User-Agent: BareSIP
```

表示请求发送端的软件信息。

仓库历史抓包中的远端：

```text
User-Agent: LinphoneAndroid/6.2.7 ...
```

不要将 User-Agent 当作可靠安全身份，它可以被任意设置。

---

## 30. `Server`

```text
Server: BareSIP
```

通常出现在 SIP Response 中，表示响应服务器或 User Agent Server 软件信息。

仓库历史响应中出现：

```text
Server: BareSIP
```

---

## 31. `Expires`

```text
Expires: 30
```

常用于 REGISTER、SUBSCRIBE 等请求，表示有效期秒数。

REGISTER 示例：

```text
Contact: <sip:1007@192.168.1.10:5060>;expires=30
Expires: 30
```

Contact 参数中的 `expires` 可针对单个 Contact；Header 级 `Expires` 可作为默认值。

注销注册常使用：

```text
Expires: 0
```

或：

```text
Contact: *
Expires: 0
```

---

## 32. `Min-Expires`

```text
Min-Expires: 60
```

当客户端请求的注册周期太短时，Registrar 可能返回：

```text
SIP/2.0 423 Interval Too Brief
Min-Expires: 60
```

客户端应使用不小于该值的周期重新注册。

当前项目 Cloud 注册周期配置为 30 秒。如果服务器要求更大的最小周期，最终行为需要结合 Baresip 的 423 处理和实际抓包确认。

---

## 33. Digest 认证 Header

### 33.1 `WWW-Authenticate`

服务器对终端进行认证挑战：

```text
SIP/2.0 401 Unauthorized
WWW-Authenticate: Digest realm="example.com",
                  nonce="...",
                  algorithm=MD5,
                  qop="auth"
```

### 33.2 `Authorization`

终端携带计算后的 Digest 响应重新请求：

```text
Authorization: Digest username="1007",
               realm="example.com",
               nonce="...",
               uri="sip:example.com",
               response="...",
               algorithm=MD5,
               qop=auth,
               nc=00000001,
               cnonce="..."
```

### 33.3 Proxy 认证

Proxy 认证使用：

```text
407 Proxy Authentication Required
Proxy-Authenticate: Digest ...
Proxy-Authorization: Digest ...
```

区别：

```text
401 -> WWW-Authenticate -> Authorization
407 -> Proxy-Authenticate -> Proxy-Authorization
```

认证 Header 含有敏感认证材料。虽然 Digest 通常不直接发送明文密码，但抓包仍不应公开传播。

---

## 34. `Authentication-Info`

```text
Authentication-Info: nextnonce="...", rspauth="..."
```

服务器可通过该 Header 返回认证结果和下一次可用 nonce 等信息。

---

## 35. `Date`

```text
Date: Wed, 28 Oct 2026 10:00:00 GMT
```

表示消息生成时间。普通 SIP 设备不一定发送。

---

## 36. `Subject`

```text
Subject: Door intercom call
```

表示呼叫主题。普通 SIP 电话通常不依赖该字段。

---

## 37. `Organization`

```text
Organization: Example Company
```

发送方组织信息，通常只用于显示或日志。

---

## 38. `Priority`

```text
Priority: normal
Priority: urgent
Priority: emergency
```

表示请求优先级。是否实际生效取决于服务器策略。

---

## 39. `Retry-After`

```text
Retry-After: 30
```

提示客户端等待指定秒数后重试，常出现在服务暂不可用等响应中。

例如：

```text
SIP/2.0 503 Service Unavailable
Retry-After: 30
```

---

## 40. `Warning`

```text
Warning: 399 proxy.example.com "Miscellaneous warning"
```

用于返回额外警告信息。程序不应只依赖可读文本判断错误类型。

---

## 41. `Reason`

```text
Reason: SIP;cause=486;text="Busy Here"
```

用于说明请求或会话结束原因，也可携带 Q.850 cause：

```text
Reason: Q.850;cause=16;text="Normal call clearing"
```

当前项目会从 Baresip 的关闭参数中推导忙线、拒接、超时等业务结束原因。

---

## 42. `Session-Expires`

```text
Session-Expires: 1800;refresher=uac
```

用于 SIP Session Timer，防止失效 Dialog 长时间残留。

相关 Header：

```text
Session-Expires
Min-SE
```

当前项目通话结束主要依赖正常 BYE、网络错误、呼叫超时和 RTP 超时；是否在运行时协商 Session Timer，应以实际 SIP 抓包为准。

---

## 43. `RSeq` 和 `RAck`

用于可靠临时响应和 PRACK。

可靠临时响应：

```text
SIP/2.0 183 Session Progress
Require: 100rel
RSeq: 1
```

PRACK：

```text
PRACK sip:... SIP/2.0
RAck: 1 1 INVITE
```

相关方法和扩展：

```text
PRACK
Supported: 100rel
Require: 100rel
RSeq
RAck
```

---

## 44. `Refer-To`

```text
Refer-To: <sip:1009@example.com>
```

用于 REFER 呼叫转移，表示希望对端联系的新目标。

相关 Header 可能包括：

```text
Refer-To
Referred-By
Replaces
```

当前项目会接收 Baresip 的 REFER/TRANSFER 相关事件，但业务是否完整支持转移需结合具体代码和运行时测试。

---

## 45. `Event`、`Allow-Events` 和 `Subscription-State`

用于 SIP Event Framework，例如 MWI、Presence。

订阅：

```text
SUBSCRIBE sip:1007@example.com SIP/2.0
Event: message-summary
```

通知：

```text
NOTIFY sip:1007@192.168.1.10 SIP/2.0
Event: message-summary
Subscription-State: active;expires=300
```

能力声明：

```text
Allow-Events: presence, message-summary
```

当前项目会收到 Baresip 的 MWI Notify 事件，但核心对讲流程不依赖这些 Header。

---

## 46. 身份与隐私扩展 Header

常见扩展：

```text
P-Asserted-Identity
P-Preferred-Identity
Privacy
History-Info
Diversion
```

### 46.1 `P-Asserted-Identity`

```text
P-Asserted-Identity: <sip:1007@example.com>
```

由受信任 SIP 网络声明已认证身份。

### 46.2 `P-Preferred-Identity`

```text
P-Preferred-Identity: <sip:1007@example.com>
```

终端希望网络使用的身份。

### 46.3 `Privacy`

```text
Privacy: id
```

请求隐藏某些身份信息。

这些字段依赖受信任网络边界，不能把来自公网任意终端的 `P-Asserted-Identity` 直接当作可信身份。

---

## 47. Header 参数、引号和显示名

示例：

```text
From: "Door 1007" <sip:1007@example.com>;tag=abc123
```

其中：

```text
"Door 1007"                 显示名
<sip:1007@example.com>       SIP URI
tag=abc123                   Header 参数
```

URI 参数与 Header 参数位置不同：

```text
Contact: <sip:1007@192.168.1.10;transport=udp>;expires=30
                                           |             |
                                           URI 参数      Header 参数
```

---

## 48. 多值 Header 与顺序

一些 Header 可以出现多次，也可以使用逗号分隔：

```text
Route: <sip:proxy1.example.com;lr>
Route: <sip:proxy2.example.com;lr>
```

或者：

```text
Route: <sip:proxy1.example.com;lr>, <sip:proxy2.example.com;lr>
```

但对于 Via、Route、Record-Route 等路由字段，值的顺序有协议意义，不能随意调整。

普通 Header 的物理排列顺序通常不重要，但 Header 与 Body 之间必须有一个空行。

---

## 49. Compact Header 紧凑写法

SIP 为部分常用 Header 定义了单字符紧凑形式。

| 完整名称 | Compact Form |
| --- | --- |
| `Via` | `v` |
| `From` | `f` |
| `To` | `t` |
| `Call-ID` | `i` |
| `Contact` | `m` |
| `Content-Type` | `c` |
| `Content-Length` | `l` |
| `Content-Encoding` | `e` |
| `Subject` | `s` |
| `Supported` | `k` |

例如：

```text
v: SIP/2.0/UDP 192.168.1.10:5060;branch=z9hG4bK-001
f: <sip:1007@example.com>;tag=abc
t: <sip:1008@example.com>
i: call-001
m: <sip:1007@192.168.1.10:5060>
c: application/sdp
l: 200
```

与完整写法语义相同。

---

## 50. SIP Transaction 与 Dialog 的区别

### 50.1 Transaction

Transaction 是一次请求及其相关响应。

例如 INVITE Transaction：

```text
INVITE
  |
  +-- 100 Trying
  +-- 180 Ringing
  +-- 200 OK
```

事务匹配重点包括：

```text
Via branch
CSeq number
CSeq method
```

### 50.2 Dialog

Dialog 是一段持续的 SIP 会话关系：

```text
INVITE -> 200 OK -> ACK
          |
          +-- Dialog 建立
                  |
                  +-- re-INVITE
                  +-- UPDATE
                  +-- INFO
                  +-- BYE
```

Dialog 标识：

```text
Call-ID + Local Tag + Remote Tag
```

关系：

```text
一个 Dialog 中可以包含多个 Transaction。
```

---

## 51. 常见 SIP 方法

| Method | 作用 |
| --- | --- |
| `REGISTER` | 注册 Contact 地址 |
| `INVITE` | 创建或修改会话 |
| `ACK` | 确认 INVITE 最终响应 |
| `CANCEL` | 取消尚未完成的 INVITE |
| `BYE` | 结束已建立的 Dialog |
| `OPTIONS` | 查询能力或保活 |
| `INFO` | 发送会话中信息 |
| `UPDATE` | 在不改变 Dialog 的情况下更新会话参数 |
| `PRACK` | 确认可靠临时响应 |
| `REFER` | 请求呼叫转移 |
| `SUBSCRIBE` | 订阅事件 |
| `NOTIFY` | 发送事件通知 |
| `MESSAGE` | 发送即时消息 |

当前项目核心通话主要涉及：

```text
REGISTER
INVITE
ACK
CANCEL
BYE
```

---

## 52. 常见响应状态码

### 52.1 1xx：临时响应

| 状态码 | 含义 |
| ---: | --- |
| `100 Trying` | 已收到请求，正在处理 |
| `180 Ringing` | 被叫正在振铃 |
| `181 Call Is Being Forwarded` | 呼叫正在转移 |
| `182 Queued` | 呼叫排队 |
| `183 Session Progress` | 会话进展，常携带早期媒体 SDP |

### 52.2 2xx：成功

| 状态码 | 含义 |
| ---: | --- |
| `200 OK` | 请求成功 |
| `202 Accepted` | 请求已接受处理 |

### 52.3 3xx：重定向

| 状态码 | 含义 |
| ---: | --- |
| `300 Multiple Choices` | 多个可选目标 |
| `301 Moved Permanently` | 永久迁移 |
| `302 Moved Temporarily` | 临时迁移 |
| `305 Use Proxy` | 使用指定 Proxy |

### 52.4 4xx：客户端请求错误

| 状态码 | 含义 |
| ---: | --- |
| `400 Bad Request` | 请求格式错误 |
| `401 Unauthorized` | UAS/Registrar 认证挑战 |
| `403 Forbidden` | 禁止请求 |
| `404 Not Found` | 找不到目标 |
| `407 Proxy Authentication Required` | Proxy 认证挑战 |
| `408 Request Timeout` | 请求超时 |
| `415 Unsupported Media Type` | 不支持 Body 类型 |
| `420 Bad Extension` | 不支持 Required 扩展 |
| `423 Interval Too Brief` | 注册周期太短 |
| `480 Temporarily Unavailable` | 临时不可用 |
| `481 Call/Transaction Does Not Exist` | Dialog 或 Transaction 不存在 |
| `482 Loop Detected` | 检测到循环 |
| `483 Too Many Hops` | Max-Forwards 耗尽 |
| `484 Address Incomplete` | 地址不完整 |
| `486 Busy Here` | 被叫忙 |
| `487 Request Terminated` | INVITE 被 CANCEL 终止 |
| `488 Not Acceptable Here` | SDP/媒体能力不可接受 |

### 52.5 5xx：服务器错误

| 状态码 | 含义 |
| ---: | --- |
| `500 Server Internal Error` | 服务器内部错误 |
| `501 Not Implemented` | 方法未实现 |
| `502 Bad Gateway` | 上游网关错误 |
| `503 Service Unavailable` | 服务不可用 |
| `504 Server Time-out` | 上游服务器超时 |
| `505 Version Not Supported` | 不支持 SIP 版本 |

### 52.6 6xx：全局失败

| 状态码 | 含义 |
| ---: | --- |
| `600 Busy Everywhere` | 所有位置均忙 |
| `603 Decline` | 明确拒绝 |
| `604 Does Not Exist Anywhere` | 全局不存在 |
| `606 Not Acceptable` | 全局媒体能力不可接受 |

---

## 53. REGISTER 报文示例

### 53.1 第一次 REGISTER

```text
REGISTER sip:<registrar-host> SIP/2.0
Via: SIP/2.0/UDP <local-ip>:5060;branch=z9hG4bK-reg-001;rport
Max-Forwards: 70
From: <sip:<username>@<domain>>;tag=<from-tag>
To: <sip:<username>@<domain>>
Call-ID: <register-call-id>
CSeq: 1 REGISTER
Contact: <sip:<username>@<local-ip>:5060;transport=udp>
Expires: 30
User-Agent: BareSIP
Content-Length: 0
```

### 53.2 401 认证挑战

```text
SIP/2.0 401 Unauthorized
Via: SIP/2.0/UDP <local-ip>:5060;branch=z9hG4bK-reg-001;rport=<source-port>;received=<source-ip>
From: <sip:<username>@<domain>>;tag=<from-tag>
To: <sip:<username>@<domain>>;tag=<server-tag>
Call-ID: <register-call-id>
CSeq: 1 REGISTER
WWW-Authenticate: Digest realm="...", nonce="...", algorithm=MD5
Content-Length: 0
```

### 53.3 携带 Authorization

```text
REGISTER sip:<registrar-host> SIP/2.0
Via: SIP/2.0/UDP <local-ip>:5060;branch=z9hG4bK-reg-002;rport
Max-Forwards: 70
From: <sip:<username>@<domain>>;tag=<from-tag>
To: <sip:<username>@<domain>>
Call-ID: <register-call-id>
CSeq: 2 REGISTER
Contact: <sip:<username>@<local-ip>:5060;transport=udp>
Expires: 30
Authorization: Digest username="...", realm="...", nonce="...", uri="...", response="..."
Content-Length: 0
```

### 53.4 注册成功

```text
SIP/2.0 200 OK
Via: ...
From: ...
To: ...;tag=<server-tag>
Call-ID: <register-call-id>
CSeq: 2 REGISTER
Contact: <sip:<username>@<local-ip>:5060;transport=udp>;expires=30
Content-Length: 0
```

---

## 54. INVITE 到 BYE 的 Header 变化

### 54.1 INVITE

```text
INVITE sip:1008@192.168.1.20 SIP/2.0
Via: SIP/2.0/UDP 192.168.1.10:5060;branch=z9hG4bK-invite-001;rport
Max-Forwards: 70
From: <sip:1007@192.168.1.10>;tag=from-001
To: <sip:1008@192.168.1.20>
Call-ID: call-001
CSeq: 1 INVITE
Contact: <sip:1007@192.168.1.10:5060>
Content-Type: application/sdp
Content-Length: <长度>

<SDP Offer>
```

初始 INVITE 的 To 通常没有 tag。

### 54.2 180 Ringing

```text
SIP/2.0 180 Ringing
Via: SIP/2.0/UDP 192.168.1.10:5060;branch=z9hG4bK-invite-001;rport=5060
From: <sip:1007@192.168.1.10>;tag=from-001
To: <sip:1008@192.168.1.20>;tag=to-001
Call-ID: call-001
CSeq: 1 INVITE
Contact: <sip:1008@192.168.1.20:5060>
Content-Length: 0
```

被叫响应中增加 To Tag。

### 54.3 200 OK

```text
SIP/2.0 200 OK
Via: SIP/2.0/UDP 192.168.1.10:5060;branch=z9hG4bK-invite-001;rport=5060
From: <sip:1007@192.168.1.10>;tag=from-001
To: <sip:1008@192.168.1.20>;tag=to-001
Call-ID: call-001
CSeq: 1 INVITE
Contact: <sip:1008@192.168.1.20:5060>
Content-Type: application/sdp
Content-Length: <长度>

<SDP Answer>
```

### 54.4 ACK

```text
ACK sip:1008@192.168.1.20:5060 SIP/2.0
Via: SIP/2.0/UDP 192.168.1.10:5060;branch=z9hG4bK-ack-001;rport
Max-Forwards: 70
From: <sip:1007@192.168.1.10>;tag=from-001
To: <sip:1008@192.168.1.20>;tag=to-001
Call-ID: call-001
CSeq: 1 ACK
Content-Length: 0
```

### 54.5 BYE

```text
BYE sip:1008@192.168.1.20:5060 SIP/2.0
Via: SIP/2.0/UDP 192.168.1.10:5060;branch=z9hG4bK-bye-001;rport
Max-Forwards: 70
From: <sip:1007@192.168.1.10>;tag=from-001
To: <sip:1008@192.168.1.20>;tag=to-001
Call-ID: call-001
CSeq: 2 BYE
Content-Length: 0
```

Dialog 标识保持不变：

```text
Call-ID = call-001
From Tag = from-001
To Tag = to-001
```

CSeq 数字增加：

```text
INVITE：1
BYE：2
```

---

## 55. CANCEL 流程

振铃阶段取消：

```text
主叫                         被叫
 |                            |
 | INVITE                     |
 |--------------------------->|
 | 180 Ringing                |
 |<---------------------------|
 |                            |
 | CANCEL                     |
 |--------------------------->|
 | 200 OK（响应 CANCEL）      |
 |<---------------------------|
 |                            |
 | 487 Request Terminated     |
 |<---------------------------|
 |                            |
 | ACK                        |
 |--------------------------->|
```

CANCEL 示例：

```text
CANCEL sip:1008@192.168.1.20 SIP/2.0
Via: SIP/2.0/UDP 192.168.1.10:5060;branch=z9hG4bK-invite-001;rport
Max-Forwards: 70
From: <sip:1007@192.168.1.10>;tag=from-001
To: <sip:1008@192.168.1.20>
Call-ID: call-001
CSeq: 1 CANCEL
Content-Length: 0
```

CANCEL 不用于结束已经建立的 Dialog。已经完成 `200 OK + ACK` 的通话应使用 BYE。

---

## 56. 当前项目的 SIP 行为

| 项目 | 当前实现 |
| --- | --- |
| SIP 协议栈 | Baresip 2.9.0 / libre |
| 传输 | UDP |
| 本地监听 | `0.0.0.0:5060` |
| LAN 账号 | Direct/P2P，不发送 REGISTER |
| Cloud 账号 | Registrar 模式 |
| Cloud 注册周期 | 30 秒 |
| 最大通话数 | 1 |
| 呼叫超时 | 30 秒 |
| 主动呼出 | `ua_connect(..., VIDMODE_OFF)`，纯音频 |
| 接听 | `ua_answer(...)` |
| 挂断 | `ua_hangup(...)` |
| 忙线 | `486 Busy Here` |
| SDP | 通过 INVITE/200 OK 等 SIP Body 传输 |

LAN URI：

```text
sip:<callee>@<LAN-IP>
```

Cloud URI：

```text
sip:<callee>@<domain>
```

---

## 57. 仓库历史 SIP 抓包解析

仓库 `docs/待处理.md` 中保存了历史 INVITE：

```text
INVITE sip:192.168.17.154:5060 SIP/2.0
Via: SIP/2.0/UDP 192.168.17.236:38989;branch=z9hG4bK.3TAt8gKAa;rport
From: "1006" <sip:1006@47.96.88.250>;tag=AVPXNHCT7
To: "1008" <sip:192.168.17.154>
CSeq: 20 INVITE
Call-ID: wiLGzGTKKC
Max-Forwards: 70
Supported: outbound
Allow: INVITE, ACK, CANCEL, OPTIONS, BYE, REFER, NOTIFY, MESSAGE, SUBSCRIBE, INFO, PRACK, UPDATE
Content-Type: application/sdp
Content-Length: 394
Contact: <sip:1006@192.168.17.236:35295;transport=udp>;expires=3600;received="sip:14.145.171.21:38175"
User-Agent: LinphoneAndroid/6.2.7 ...
```

逐项说明：

| 字段 | 含义 |
| --- | --- |
| `INVITE sip:192.168.17.154:5060 SIP/2.0` | 请求目标是设备 SIP 5060 端口 |
| `Via` | 响应返回路径和 INVITE Transaction branch |
| `rport` | 请求按实际 UDP 源端口返回响应 |
| `From` | 逻辑主叫号码 1006，并携带主叫 Tag |
| `To` | 逻辑被叫 1008，初始 INVITE 尚无 To Tag |
| `CSeq: 20 INVITE` | 第 20 号 INVITE 序列 |
| `Call-ID` | 此次呼叫标识 |
| `Max-Forwards: 70` | 最大 SIP 转发跳数 |
| `Supported: outbound` | 支持 SIP Outbound 扩展 |
| `Allow` | 远端声明支持的方法 |
| `Content-Type: application/sdp` | Body 是 SDP |
| `Content-Length: 394` | SDP Body 长度 |
| `Contact` | 后续 SIP 请求应发送到的远端地址 |
| `User-Agent` | 远端软件是 Linphone Android |

设备当时返回：

```text
SIP/2.0 404 Not Found
Via: SIP/2.0/UDP 192.168.17.236:38989;
     branch=z9hG4bK.3TAt8gKAa;
     rport=35295;
     received=192.168.17.236
From: "1006" <sip:1006@47.96.88.250>;tag=AVPXNHCT7
To: "1008" <sip:192.168.17.154>;tag=e311a90259ae7e33
CSeq: 20 INVITE
Call-ID: wiLGzGTKKC
Server: BareSIP
Content-Length: 0
```

响应特点：

```text
Via branch 不变                 -> 对应原 INVITE Transaction
From Tag 不变                   -> 保持主叫标识
To 增加 tag                     -> 被叫侧响应 Tag
Call-ID 不变                    -> 对应同一呼叫
CSeq 不变且 Method 仍是 INVITE -> 响应原 INVITE
Content-Length: 0               -> 没有 Body
```

主叫随后发送 ACK，结束这次非 2xx INVITE Transaction。

---

## 58. SIP、SDP、RTP 地址对比

```text
Via         -> SIP 响应返回和事务路由
Route       -> SIP 请求经过的路由
Record-Route-> 建立 Dialog Route Set
Contact     -> 后续 SIP 请求目标
Request-URI -> 当前 SIP 请求目标
SDP c=      -> RTP/RTCP 目标 IP
SDP m=      -> RTP/RTCP 目标端口
```

示例：

```text
SIP Proxy：47.96.88.250:25060
设备 SIP：192.168.1.10:5060
设备音频 RTP：192.168.1.10:10000
设备视频 RTP：192.168.1.10:10002
```

不要使用 Via、Contact 或 Request-URI 推导 RTP 地址。RTP 连接应根据 SDP 的 `c=` 和 `m=` 判断。

---

## 59. SIP Header 排查清单

### 59.1 请求基本合法性

```text
[ ] Request-Line 是否包含正确 Method、Request-URI、SIP/2.0
[ ] Via 是否存在 branch=z9hG4bK...
[ ] Max-Forwards 是否大于 0
[ ] From 是否有合法 URI 和 From Tag
[ ] To URI 是否正确
[ ] Call-ID 是否存在
[ ] CSeq Method 是否与 Request-Line Method 一致
[ ] Content-Length 是否与 Body 长度一致
```

### 59.2 Dialog

```text
[ ] INVITE、200 OK、ACK、BYE 的 Call-ID 是否一致
[ ] From Tag 是否一致
[ ] To Tag 是否一致
[ ] BYE 的 CSeq 是否比初始 INVITE 大
[ ] Contact 和 Route Set 是否正确
```

### 59.3 REGISTER

```text
[ ] Request-URI 是否指向 Registrar
[ ] To/From AOR 是否正确
[ ] Contact 是否为设备可达地址
[ ] Expires 是否符合服务器 Min-Expires
[ ] 401 后是否携带 Authorization 重发
[ ] CSeq 是否递增
[ ] 200 OK 是否返回有效 Contact/Expires
```

### 59.4 INVITE/SDP

```text
[ ] Content-Type 是否为 application/sdp
[ ] Content-Length 是否正确
[ ] Offer 与 Answer 是否都存在
[ ] 200 OK 后是否发送 ACK
[ ] SDP c=/m= 是否给出可达 RTP 地址
```

### 59.5 NAT

```text
[ ] Via 是否携带 rport
[ ] 响应 Via 是否出现 received/rport
[ ] Contact 是否携带不可达私网地址
[ ] SDP c= 是否携带不可达私网地址
[ ] SIP 正常但 RTP 不通时，不要只检查 5060
```

---

## 60. 抓包命令

只抓 SIP：

```bash
tcpdump -ni any -s0 -vvv -A 'udp port 5060'
```

抓当前项目 SIP 和 RTP：

```bash
tcpdump -ni any -s0 -w /tmp/sip_call.pcap 'udp port 5060 or udp portrange 10000-10100'
```

Wireshark 过滤器：

```text
sip
sip.Method == "INVITE"
sip.Method == "REGISTER"
sip.Status-Code == 200
sip.Status-Code >= 400
sip.Call-ID == "<call-id>"
sdp
rtp
rtcp
sip || rtp || rtcp
```

检查单次呼叫时，优先使用 `Call-ID` 过滤整段 SIP 对话。

---

## 61. SIP Header 参考链接

### 61.1 核心规范

- SIP 核心规范 RFC 3261：<https://datatracker.ietf.org/doc/html/rfc3261>
- IANA SIP Parameters 注册表：<https://www.iana.org/assignments/sip-parameters/sip-parameters.xhtml>
- SIP URI 方案注册：<https://www.iana.org/assignments/uri-schemes/prov/sip>

### 61.2 NAT、路由和连接

- SIP `rport` RFC 3581：<https://datatracker.ietf.org/doc/html/rfc3581>
- Path Header RFC 3327：<https://datatracker.ietf.org/doc/html/rfc3327>
- Service-Route RFC 3608：<https://datatracker.ietf.org/doc/html/rfc3608>
- SIP Outbound RFC 5626：<https://datatracker.ietf.org/doc/html/rfc5626>

### 61.3 呼叫和会话扩展

- Reliable Provisional Response / PRACK RFC 3262：<https://datatracker.ietf.org/doc/html/rfc3262>
- UPDATE RFC 3311：<https://datatracker.ietf.org/doc/html/rfc3311>
- Reason Header RFC 3326：<https://datatracker.ietf.org/doc/html/rfc3326>
- REFER RFC 3515：<https://datatracker.ietf.org/doc/html/rfc3515>
- Replaces Header RFC 3891：<https://datatracker.ietf.org/doc/html/rfc3891>
- SIP Session Timer RFC 4028：<https://datatracker.ietf.org/doc/html/rfc4028>

### 61.4 身份、隐私和事件

- P-Asserted-Identity RFC 3325：<https://datatracker.ietf.org/doc/html/rfc3325>
- SIP Event Framework RFC 6665：<https://datatracker.ietf.org/doc/html/rfc6665>
- Message Waiting Indication RFC 3842：<https://datatracker.ietf.org/doc/html/rfc3842>

### 61.5 SDP 和 RTP

- SDP RFC 8866：<https://datatracker.ietf.org/doc/html/rfc8866>
- SDP Offer/Answer RFC 3264：<https://datatracker.ietf.org/doc/html/rfc3264>
- RTP RFC 3550：<https://datatracker.ietf.org/doc/html/rfc3550>
- RTP/AVP RFC 3551：<https://datatracker.ietf.org/doc/html/rfc3551>
- H.264 RTP Payload RFC 6184：<https://datatracker.ietf.org/doc/html/rfc6184>

---

## 62. 核心结论

```text
SIP Start-Line：说明这是哪个请求或哪个响应

SIP Headers：
    - 标识事务
    - 标识 Dialog
    - 指定 SIP 路由
    - 声明逻辑身份
    - 指定后续请求地址
    - 完成认证
    - 描述消息体类型和长度

SIP Body：
    - 可携带 SDP
    - SDP 协商 RTP 媒体参数
```

最重要的标识关系：

```text
Transaction：重点看 Via branch + CSeq

Dialog：Call-ID + Local Tag + Remote Tag
```

最重要的地址区别：

```text
Via / Route / Contact / Request-URI：SIP 信令地址

SDP c= + m=：RTP/RTCP 媒体地址
```

当前项目典型协议关系：

```text
SIP UDP 5060
 |
 +-- REGISTER：Cloud 账号注册
 +-- INVITE/200/ACK：建立通话并通过 Body 协商 SDP
 +-- BYE/200：结束通话

SDP
 |
 +-- 协商 G.711 音频 RTP
 +-- 协商 H.264 视频 RTP

RTP 10000~10100
 |
 +-- 音频：PCMU/PCMA
 +-- 视频：H.264
```
