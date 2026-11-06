# SDP 报文结构与字段说明

## 1. 文档目的

本文介绍 SDP（Session Description Protocol，会话描述协议）的完整结构、字段含义、Offer/Answer 协商方式，以及 SDP 如何确定 RTP/RTCP 的目标地址和端口。

本文同时结合当前项目说明：

- G.711 音频 SDP；
- H.264 视频 SDP；
- 音频和视频的独立 RTP 通道；
- `rtcp_mux` 的含义；
- 当前项目视频 `recvonly` 的方向；
- 仓库历史抓包中的 SDP；
- 如何通过 SDP 排查 RTP 连接问题。

说明：SDP 的固定字段可以完整列出，但 `a=` 属性是可扩展的。不同 codec、SIP 客户端、WebRTC 实现和厂商可以定义不同属性，因此不存在一份能够覆盖所有私有 `a=` 属性的有限列表。

---

## 2. SDP 是什么

SDP 本身不负责建立通话，也不传输音频和视频数据。它只描述一次媒体会话，例如：

```text
媒体使用什么 IP？
媒体使用什么 UDP 端口？
支持哪些音频和视频 codec？
RTP Payload Type 如何映射到 codec？
媒体方向是 sendrecv、sendonly 还是 recvonly？
H.264 使用什么 profile 和 RTP 分包模式？
RTP 与 RTCP 是否复用端口？
```

在当前项目的 SIP 通话中，SDP 作为 SIP Message Body 传输：

```text
Ethernet
└── IP
    └── UDP
        └── SIP
            ├── Request Line / Status Line
            ├── SIP Headers
            ├── 空行
            └── SDP Body（可选）
```

SIP Header 通过以下字段声明消息体是 SDP：

```text
Content-Type: application/sdp
Content-Length: <SDP字节长度>
```

示例：

```text
INVITE sip:1008@192.168.1.20 SIP/2.0
Via: SIP/2.0/UDP 192.168.1.10:5060;branch=z9hG4bK...
From: <sip:1007@192.168.1.10>;tag=abc123
To: <sip:1008@192.168.1.20>
Call-ID: call-001
CSeq: 1 INVITE
Content-Type: application/sdp
Content-Length: <SDP长度>

v=0
o=- 10001 10001 IN IP4 192.168.1.10
s=-
c=IN IP4 192.168.1.10
t=0 0
m=audio 10000 RTP/AVP 0 8
a=rtpmap:0 PCMU/8000
a=rtpmap:8 PCMA/8000
a=sendrecv
a=rtcp-mux
```

空行之前属于 SIP，空行之后以 `v=0` 开始的内容属于 SDP。

---

## 3. SDP 与 RTP 的关系

建立阶段：

```text
UDP 5060
└── SIP
    └── SDP：协商 RTP 如何传输
```

通话阶段：

```text
UDP <audio-port>
└── RTP
    └── G.711 音频 Payload

UDP <video-port>
└── RTP
    └── H.264 视频 Payload
```

因此，SDP 不包含实际音视频数据，RTP 也不是封装在 SDP 里面。

错误理解：

```text
SIP
└── SDP
    └── RTP
        └── 音视频数据
```

正确关系：

```text
SIP + SDP：建立会话并协商媒体参数

RTP：根据协商结果，独立传输音视频数据
```

对应时序：

```text
主叫                                           被叫
  |                                              |
  |  SIP INVITE                                  |
  |  Body = SDP Offer                            |
  |--------------------------------------------->|
  |                                              |
  |  SIP 200 OK                                  |
  |  Body = SDP Answer                           |
  |<---------------------------------------------|
  |                                              |
  |  SIP ACK                                     |
  |--------------------------------------------->|
  |                                              |
  |<========= RTP 音频/视频独立传输 ============>|
  |                                              |
  |  SIP BYE                                     |
  |--------------------------------------------->|
  |                                              |
  |  SIP 200 OK                                  |
  |<---------------------------------------------|
```

---

## 4. SDP 的层次结构

SDP 可以分为两层：

```text
SDP
 |
 +-- Session-level：整个会话的公共描述
 |
 +-- Media-level：某一路媒体的描述
        |
        +-- m=audio
        |    └── 音频属性
        |
        +-- m=video
             └── 视频属性
```

一个 SDP 可以有多个 `m=` 段。每个 `m=` 开始一段新的媒体描述，该段一直持续到下一个 `m=` 或 SDP 结束。

例如：

```text
v=0                              <- Session-level
o=- 10001 10002 IN IP4 192.168.1.10
s=Talk
c=IN IP4 192.168.1.10
t=0 0
a=sendrecv

m=audio 10000 RTP/AVP 0 8       <- Audio media-level 开始
a=rtpmap:0 PCMU/8000
a=rtpmap:8 PCMA/8000
a=ptime:20
a=rtcp-mux
a=sendrecv

m=video 10002 RTP/AVP 96        <- Video media-level 开始
a=rtpmap:96 H264/90000
a=fmtp:96 packetization-mode=1;profile-level-id=42801f
a=rtcp-mux
a=recvonly
```

Session-level 的属性默认适用于所有媒体；Media-level 属性只作用于当前 `m=` 段，并可覆盖会话级设置。

---

## 5. SDP 完整字段结构

标准 SDP 的排列结构大致如下：

```text
v=                         必须，SDP 版本
o=                         必须，会话发起者和会话版本
s=                         必须，会话名称
i=                         可选，会话说明
u=                         可选，会话信息 URI
e=                         可选，联系邮箱
p=                         可选，联系电话
c=                         可选，会话级连接地址
b=                         可选，会话级带宽
t=                         必须，会话时间
r=                         可选，重复时间
z=                         可选，时区调整
k=                         已废弃，旧式加密密钥
a=                         可选，会话级属性

m=                         媒体描述开始
i=                         可选，媒体说明
c=                         可选，媒体级连接地址
b=                         可选，媒体级带宽
k=                         已废弃，媒体级旧式密钥
a=                         可选，媒体级属性

m=                         下一路媒体
...
```

SIP 实时通话中最常见的字段是：

```text
v=  o=  s=  c=  t=
m=audio / m=video
a=rtpmap
a=fmtp
a=sendrecv / sendonly / recvonly / inactive
a=ptime / maxptime
a=rtcp / rtcp-mux
a=rtcp-fb
```

---

## 6. `v=`：SDP 版本

```text
v=0
```

格式：

```text
v=<version>
```

当前 SDP 协议版本固定为 `0`。它不是应用版本，也不是 SIP 版本。

---

## 7. `o=`：Origin，会话来源

示例：

```text
o=- 10001 10002 IN IP4 192.168.1.10
```

格式：

```text
o=<username> <session-id> <session-version> <nettype> <addrtype> <address>
```

| 字段 | 示例 | 含义 |
| --- | --- | --- |
| `username` | `-` | 会话创建者，未知时常使用 `-` |
| `session-id` | `10001` | 会话 ID |
| `session-version` | `10002` | SDP 内容版本 |
| `nettype` | `IN` | Internet |
| `addrtype` | `IP4` | IPv4 |
| `address` | `192.168.1.10` | 会话来源地址 |

SDP 内容发生变化时，通常保持 `session-id` 不变并增加 `session-version`：

```text
第一次：

o=- 10001 10001 IN IP4 192.168.1.10

重新协商：

o=- 10001 10002 IN IP4 192.168.1.10
```

`o=` 中的 IP 主要用于标识 SDP 来源，不一定是最终 RTP 目标地址。RTP 目标地址主要看 `c=`。

---

## 8. `s=`：Session Name，会话名称

```text
s=Talk
```

或者：

```text
s=-
```

`s=` 是必须字段，但 SIP 电话通常不依赖其内容。

---

## 9. `i=`：会话或媒体说明

会话级：

```text
i=Video intercom session
```

媒体级：

```text
m=video 10002 RTP/AVP 96
i=Door camera
```

`i=` 是人类可读说明，通常不参与 codec 协商。

---

## 10. `u=`：会话信息 URI

```text
u=https://example.com/session/10001
```

表示与会话有关的信息地址。普通 SIP 电话中很少使用。

---

## 11. `e=` 和 `p=`：联系人

```text
e=admin@example.com
p=+86-123456789
```

分别表示联系邮箱和联系电话。当前 SIP 对讲项目通常不使用这些字段。

---

## 12. `c=`：Connection，媒体连接地址

`c=` 是 SDP 中最重要的字段之一。

```text
c=IN IP4 192.168.1.10
```

格式：

```text
c=<nettype> <addrtype> <connection-address>
```

| 字段 | 示例 | 含义 |
| --- | --- | --- |
| `nettype` | `IN` | Internet |
| `addrtype` | `IP4` | IPv4 |
| `connection-address` | `192.168.1.10` | RTP/RTCP 媒体地址 |

### 12.1 `c=` 与 `m=` 共同确定 RTP 目标

```text
c=IN IP4 192.168.1.10

m=audio 10000 RTP/AVP 8
m=video 10002 RTP/AVP 96
```

表示：

```text
音频 RTP 目标：192.168.1.10:10000
视频 RTP 目标：192.168.1.10:10002
```

```text
c=IN IP4 192.168.1.10
          |
          +-- m=audio 10000 -> 192.168.1.10:10000
          |
          +-- m=video 10002 -> 192.168.1.10:10002
```

### 12.2 Session-level `c=`

`c=` 位于第一个 `m=` 之前时，对所有没有独立 `c=` 的媒体生效：

```text
v=0
o=- 10001 10001 IN IP4 192.168.1.10
s=-
c=IN IP4 192.168.1.10
t=0 0

m=audio 10000 RTP/AVP 8
m=video 10002 RTP/AVP 96
```

结果：

```text
audio -> 192.168.1.10:10000
video -> 192.168.1.10:10002
```

### 12.3 Media-level `c=`

每一路媒体也可以设置自己的连接地址：

```text
c=IN IP4 192.168.1.10

m=audio 10000 RTP/AVP 8
c=IN IP4 192.168.1.11

m=video 10002 RTP/AVP 96
c=IN IP4 192.168.1.12
```

结果：

```text
audio -> 192.168.1.11:10000
video -> 192.168.1.12:10002
```

Media-level `c=` 覆盖 Session-level `c=`。

### 12.4 RTP 连接判断公式

```text
RTP 目标地址 = 当前媒体生效的 c= 地址 + 当前 m= 端口
```

如果媒体级存在 `c=`，使用媒体级地址；否则继承会话级 `c=`。

---

## 13. `b=`：Bandwidth，带宽

示例：

```text
b=AS:512
```

常见类型：

```text
b=AS:512
b=TIAS:500000
```

| 类型 | 含义 |
| --- | --- |
| `AS` | Application Specific，单位通常为 Kbit/s |
| `TIAS` | Transport Independent Application Specific，单位为 bit/s |
| `CT` | Conference Total |

媒体级示例：

```text
m=video 10002 RTP/AVP 96
b=AS:512
```

表示该视频媒体声明约 512 Kbit/s 带宽。

当前项目在 Baresip 配置中设置了 RTP 接收带宽上限：

```c
config->avt.rtp_bw.max = 512U * 1000U;
```

最终是否生成 `b=` 需要以运行时 SDP 抓包为准。

---

## 14. `t=`：会话时间

```text
t=0 0
```

格式：

```text
t=<start-time> <stop-time>
```

SIP 通话中通常使用 `t=0 0`，表示不指定固定的开始和结束时间，不表示通话时长为 0。

---

## 15. `r=`：重复时间

```text
r=7d 1h 0 25h
```

主要用于定时广播、会议等重复会话。普通 SIP 实时通话基本不使用。

---

## 16. `z=`：时区调整

```text
z=2882844526 -1h 2898848070 0
```

用于定时会话的时区或夏令时调整。普通 SIP 实时通话基本不使用。

---

## 17. `k=`：旧式密钥描述

旧式示例：

```text
k=clear:password
k=base64:...
k=uri:https://example.com/key
k=prompt
```

`k=` 已废弃，不应在新实现中用于保护媒体。现代安全媒体通常使用 SRTP、SDES、DTLS-SRTP 或其他密钥协商机制。

当前项目使用 `RTP/AVP`，没有在当前配置中启用可确认的 SRTP 媒体加密。

---

## 18. `m=`：Media，媒体描述

格式：

```text
m=<media> <port>[/<port-count>] <proto> <fmt> ...
```

### 18.1 音频

```text
m=audio 10000 RTP/AVP 0 8
```

| 字段 | 值 | 含义 |
| --- | --- | --- |
| `media` | `audio` | 音频 |
| `port` | `10000` | 音频 RTP 接收端口 |
| `proto` | `RTP/AVP` | RTP Audio/Video Profile |
| `fmt` | `0 8` | 支持 PT 0 和 PT 8 |

### 18.2 视频

```text
m=video 10002 RTP/AVP 96
```

表示：

```text
媒体类型：视频
RTP 接收端口：10002
传输协议：RTP/AVP
Payload Type：96
```

其他可能的媒体类型：

```text
m=audio
m=video
m=text
m=application
m=message
```

当前项目主要关心 `audio` 和 `video`。

### 18.3 端口为 0

Answer 中将端口设置为 0，表示拒绝或禁用该路媒体：

```text
m=video 0 RTP/AVP 96
a=inactive
```

例如，Offer 提供音频和视频：

```text
m=audio 40000 RTP/AVP 0 8
m=video 40002 RTP/AVP 96
```

纯音频 Answer：

```text
m=audio 10000 RTP/AVP 8
a=sendrecv

m=video 0 RTP/AVP 96
a=inactive
```

含义：

```text
音频：接受
视频：拒绝
```

---

## 19. `a=`：Attribute，属性

`a=` 是 SDP 中最灵活、最常见的字段。

格式：

```text
a=<attribute>
```

或者：

```text
a=<attribute>:<value>
```

例如：

```text
a=sendrecv
a=rtcp-mux
a=rtpmap:96 H264/90000
a=fmtp:96 packetization-mode=1
```

`a=` 可以出现在 Session-level 或 Media-level。Media-level 属性只对当前 `m=` 段生效。

---

## 20. `a=rtpmap`：Payload Type 与 codec 映射

格式：

```text
a=rtpmap:<PT> <encoding-name>/<clock-rate>[/<channels>]
```

### 20.1 PCMU

```text
a=rtpmap:0 PCMU/8000
```

表示：

```text
PT = 0
codec = PCMU
RTP clock = 8000 Hz
channels = 默认 1
```

### 20.2 PCMA

```text
a=rtpmap:8 PCMA/8000
```

表示：

```text
PT = 8
codec = PCMA
RTP clock = 8000 Hz
```

### 20.3 Opus

```text
a=rtpmap:96 opus/48000/2
```

表示：

```text
PT = 96
codec = Opus
RTP clock = 48000 Hz
channels = 2
```

当前项目只加载 G.711 codec，不加载 Opus。远端即使提供 Opus，本机也只能从双方共同支持的 PCMU/PCMA 中选择。

### 20.4 H.264

```text
a=rtpmap:96 H264/90000
```

表示：

```text
PT = 96
codec = H.264
RTP clock = 90000 Hz
```

`96` 是动态 PT，只在当前 SDP 协商中有效，不能假定 PT 96 永远对应 H.264。

---

## 21. `a=fmtp`：codec 格式参数

格式：

```text
a=fmtp:<PT> <format-parameters>
```

### 21.1 Opus 示例

```text
a=fmtp:96 useinbandfec=1
```

表示 Opus 启用带内前向纠错。

### 21.2 H.264 示例

```text
a=fmtp:96 packetization-mode=1;profile-level-id=42801f
```

含义：

```text
PT = 96
packetization-mode = 1
profile-level-id = 42801f
```

当前项目 H.264 主要能力：

```text
packetization-mode=1
profile-level-id=42801f
```

如果 SDP 携带 SPS/PPS：

```text
a=fmtp:96 packetization-mode=1;
          profile-level-id=42801f;
          sprop-parameter-sets=<Base64-SPS>,<Base64-PPS>
```

项目可以从 `sprop-parameter-sets` 解析 SPS/PPS。如果 SDP 不携带参数集，项目会等待 RTP 中的内联 SPS/PPS。

---

## 22. 媒体方向属性

### 22.1 `a=sendrecv`

```text
a=sendrecv
```

声明方发送并接收媒体。

### 22.2 `a=sendonly`

```text
a=sendonly
```

声明方只发送、不接收。

### 22.3 `a=recvonly`

```text
a=recvonly
```

声明方只接收、不发送。

### 22.4 `a=inactive`

```text
a=inactive
```

声明方既不发送也不接收。

### 22.5 当前项目方向

当前设备接听视频来电时：

```text
m=audio 10000 RTP/AVP 8
a=sendrecv

m=video 10002 RTP/AVP 96
a=recvonly
```

含义：

```text
音频：本机发送并接收
视频：本机只接收，不发送
```

实际数据方向：

```text
远端设备                                      本设备
    |                                            |
    |========== G.711 音频 RTP =================>|
    |<========= G.711 音频 RTP ==================|
    |                                            |
    |========== H.264 视频 RTP =================>|
    |                                            |
    |       本设备不会发送 H.264 视频 RTP         |
```

---

## 23. `a=ptime` 和 `a=maxptime`

```text
a=ptime:20
```

表示建议每个音频 RTP 包承载 20 ms 音频。

对于 G.711：

```text
8000 sample/s * 20 ms = 160 samples
```

所以典型 G.711 Payload 是 160 字节。

```text
a=maxptime:60
```

表示接收方支持的最大 packet time 为 60 ms。

---

## 24. `a=rtcp`

RTP 和 RTCP 不复用端口时，可以显式声明 RTCP 端口：

```text
c=IN IP4 192.168.1.10
m=audio 10000 RTP/AVP 8
a=rtcp:10001
```

连接为：

```text
音频 RTP  -> 192.168.1.10:10000
音频 RTCP -> 192.168.1.10:10001
```

也可以显式携带地址：

```text
a=rtcp:10001 IN IP4 192.168.1.10
```

---

## 25. `a=rtcp-mux`

```text
a=rtcp-mux
```

表示当前媒体的 RTP 和 RTCP 使用同一个 UDP 端口：

```text
m=audio 10000 RTP/AVP 8
a=rtcp-mux
```

连接为：

```text
音频 RTP  -> 192.168.1.10:10000
音频 RTCP -> 192.168.1.10:10000
```

视频仍然有自己的媒体端口：

```text
m=video 10002 RTP/AVP 96
a=rtcp-mux
```

```text
视频 RTP  -> 192.168.1.10:10002
视频 RTCP -> 192.168.1.10:10002
```

所以：

```text
rtcp-mux = 同一路媒体的 RTP/RTCP 复用
```

不是：

```text
音频 RTP 和视频 RTP 复用
```

当前项目设置：

```c
config->avt.rtcp_mux = true;
```

典型结构：

```text
              +-- UDP 10000：音频 RTP + 音频 RTCP
SIP 通话 -----|
              +-- UDP 10002：视频 RTP + 视频 RTCP
```

---

## 26. `a=rtcp-fb`：RTCP Feedback

示例：

```text
a=rtcp-fb:* trr-int 1000
a=rtcp-fb:* ccm tmmbr
```

H.264 常见反馈：

```text
a=rtcp-fb:96 nack
a=rtcp-fb:96 nack pli
a=rtcp-fb:96 ccm fir
```

| 属性 | 含义 |
| --- | --- |
| `nack` | 请求重传丢失 RTP 包 |
| `nack pli` | Picture Loss Indication，请求新图像或关键帧 |
| `ccm fir` | Full Intra Request，请求完整关键帧 |
| `ccm tmmbr` | 请求临时最大媒体码率 |

远端在 SDP 中声明某项反馈能力，不代表当前项目一定实现了对应处理，需要结合 Baresip codec 和运行时行为判断。

---

## 27. `a=rtcp-xr`：扩展 RTCP 报告

仓库历史抓包中出现：

```text
a=rtcp-xr:rcvr-rtt=all:10000 stat-summary=loss,dup,jitt,TTL voip-metrics
```

它表示支持 RTCP Extended Reports，包括：

- RTT；
- 丢包；
- 重复包；
- jitter；
- TTL；
- VoIP 质量指标。

该属性主要用于媒体质量统计和诊断。

---

## 28. DTMF：`telephone-event`

示例：

```text
a=rtpmap:101 telephone-event/48000
a=rtpmap:97 telephone-event/8000
```

表示通过 RTP Event 传输 DTMF 按键，而不是直接发送按键声音。

例如：

```text
m=audio 40481 RTP/AVP 96 0 8 101 97
```

映射可能为：

```text
96  -> Opus
0   -> PCMU
8   -> PCMA
101 -> 48 kHz telephone-event
97  -> 8 kHz telephone-event
```

当前项目是否协商对应 DTMF PT，需要以 Baresip 运行时 SDP 和 codec 列表为准。

---

## 29. 其他常见 SDP 属性

以下属性在其他 SIP/WebRTC 系统中常见，但不属于当前项目核心媒体路径。

### 29.1 ICE

```text
a=ice-ufrag:...
a=ice-pwd:...
a=candidate:...
a=end-of-candidates
```

用于 NAT 穿透和连接候选地址协商。

### 29.2 DTLS-SRTP

```text
a=fingerprint:sha-256 ...
a=setup:actpass
```

用于协商 DTLS 和 SRTP 密钥。

### 29.3 SDES-SRTP

```text
a=crypto:1 AES_CM_128_HMAC_SHA1_80 inline:...
```

用于通过 SDP 交换 SRTP 密钥材料。

### 29.4 BUNDLE

```text
a=group:BUNDLE 0 1
a=mid:0
a=mid:1
```

允许多路媒体共享传输通道，常用于 WebRTC。当前项目的典型 SIP/RTP 路径仍使用独立音频和视频媒体端口，不能将 BUNDLE 行为套用到当前项目。

### 29.5 RTP Header Extension

```text
a=extmap:1 <extension-uri>
```

用于协商 RTP Header Extension。

### 29.6 SSRC

```text
a=ssrc:12345678 cname:stream@example.com
```

描述 RTP 同步源和相关属性。

---

## 30. 音频和视频是否在同一个通道

当前项目中，音频和视频是两路独立 RTP 流：

```text
                         一次 SIP 通话
                              |
                +-------------+-------------+
                |                           |
                v                           v
           音频 RTP 流                  视频 RTP 流
         UDP <audio-port>              UDP <video-port>
                |                           |
                v                           v
       RTP + G.711 Payload          RTP + H.264 Payload
```

H.264 只编码视频，不包含音频：

```text
视频编码：H.264
音频编码：G.711
```

MP4、MKV、MPEG-TS 等容器可以同时包含音频轨和视频轨，例如：

```text
MP4 容器
 |
 +-- Video Track
 |      └── H.264
 |
 +-- Audio Track
        └── AAC
```

这是容器同时封装两条轨道，不是 H.264 内部包含 AAC。当前项目不使用 MP4 或 MPEG-TS 承载通话媒体，而是分别发送 G.711 RTP 和 H.264 RTP。

---

## 31. SDP Offer/Answer 协商

### 31.1 Offer

Offer 声明发送方可以提供或接收的媒体能力：

```text
v=0
o=- 10001 10001 IN IP4 192.168.1.20
s=-
c=IN IP4 192.168.1.20
t=0 0

m=audio 40000 RTP/AVP 0 8
a=rtpmap:0 PCMU/8000
a=rtpmap:8 PCMA/8000
a=sendrecv
a=rtcp-mux

m=video 40002 RTP/AVP 96
a=rtpmap:96 H264/90000
a=fmtp:96 packetization-mode=1;profile-level-id=42801f
a=sendonly
a=rtcp-mux
```

### 31.2 Answer

Answer 从 Offer 中选择双方兼容的格式并声明自己的接收地址：

```text
v=0
o=- 20001 20001 IN IP4 192.168.1.10
s=-
c=IN IP4 192.168.1.10
t=0 0

m=audio 10000 RTP/AVP 8
a=rtpmap:8 PCMA/8000
a=sendrecv
a=rtcp-mux

m=video 10002 RTP/AVP 96
a=rtpmap:96 H264/90000
a=fmtp:96 packetization-mode=1;profile-level-id=42801f
a=recvonly
a=rtcp-mux
```

协商结果：

```text
音频：PCMA，双向
视频：H.264，远端发送，本机接收
```

### 31.3 Payload Type 关系

Answer 对 Offer 中某一动态格式作出响应时，需要保持该格式在 Offer 中使用的 PT 映射。例如 Offer 中：

```text
a=rtpmap:96 H264/90000
```

Answer 接受该格式时继续使用 PT 96：

```text
m=video 10002 RTP/AVP 96
a=rtpmap:96 H264/90000
```

动态 PT 只在当前 SDP 协商上下文中有意义。

---

## 32. 如何根据 Offer/Answer 建立 RTP 连接

### 32.1 对端 Offer

对端发送：

```text
c=IN IP4 192.168.1.20

m=audio 40000 RTP/AVP 0 8
a=rtcp-mux

m=video 40002 RTP/AVP 96
a=rtcp-mux
```

本机从这个 SDP 得到：

```text
本机发送音频的目标：192.168.1.20:40000
本机发送视频的目标：192.168.1.20:40002
```

当前设备视频为 `recvonly`，因此不会向对端发送视频 RTP。

### 32.2 本机 Answer

本机回复：

```text
c=IN IP4 192.168.1.10

m=audio 10000 RTP/AVP 8
a=sendrecv
a=rtcp-mux

m=video 10002 RTP/AVP 96
a=recvonly
a=rtcp-mux
```

对端从本机 SDP 得到：

```text
对端发送音频的目标：192.168.1.10:10000
对端发送视频的目标：192.168.1.10:10002
```

### 32.3 完整连接图

```text
本机 192.168.1.10                         对端 192.168.1.20
      |                                        |
      |  G.711 RTP                             |
      |  dst=192.168.1.20:40000                |
      |--------------------------------------->|
      |                                        |
      |  G.711 RTP                             |
      |  dst=192.168.1.10:10000                |
      |<---------------------------------------|
      |                                        |
      |  H.264 RTP                             |
      |  dst=192.168.1.10:10002                |
      |<---------------------------------------|
      |                                        |
      |  本机 video=recvonly，不发送视频        |
```

---

## 33. SIP 地址与 RTP 地址的区别

以下地址用途不同：

```text
Via         = SIP 响应返回和事务路由信息
Contact     = 后续 SIP 对话内请求地址
Request-URI = 当前 SIP 请求目标
SDP c=      = RTP/RTCP 媒体目标地址
SDP m=      = RTP/RTCP 媒体端口
```

例如：

```text
SIP Proxy：47.96.88.250:25060
设备 SIP：192.168.1.10:5060
设备 RTP：192.168.1.10:10000
```

SIP 发给 Proxy，不代表 RTP 也发送给 Proxy。RTP 通常根据 SDP 中的 `c=` 和 `m=` 直达媒体端点；如果部署了媒体代理，则 SDP 中的地址可能指向媒体代理。

---

## 34. 音视频 RTP 的关联和同步

音频和视频各自拥有独立的：

- UDP 端口；
- Payload Type；
- SSRC；
- Sequence Number；
- Timestamp；
- RTP 时钟。

例如：

```text
音频 RTP：
    SSRC      = 0x11111111
    PT        = 8
    Clock     = 8000 Hz
    Timestamp = 160000

视频 RTP：
    SSRC      = 0x22222222
    PT        = 96
    Clock     = 90000 Hz
    Timestamp = 900000
```

音频和视频 Timestamp 不能直接进行数值比较，因为时钟频率不同。标准 RTP 系统一般通过 RTCP Sender Report 中的 NTP 时间与 RTP Timestamp 映射，将两路媒体关联到共同时间轴：

```text
音频 RTCP SR：NTP 时间 <-> 音频 RTP Timestamp
视频 RTCP SR：NTP 时间 <-> 视频 RTP Timestamp
```

当前项目的自定义 H.264 硬件重组器主要使用 Sequence 和 Marker 重组 Access Unit，没有在该 decoder 层使用 RTP Timestamp 做音视频同步。

---

## 35. 当前项目 SDP 能力

当前项目的重要配置：

| 项目 | 当前实现 |
| --- | --- |
| SIP 传输 | UDP |
| SIP 本地监听 | `0.0.0.0:5060` |
| RTP 端口范围 | `10000~10100` |
| RTP/RTCP | 启用 `rtcp_mux` |
| 音频 codec | PCMU/PT=0、PCMA/PT=8 |
| 音频 RTP 时钟 | 8000 Hz |
| 视频 codec | H.264/90000 |
| H.264 分包模式 | 主要使用 `packetization-mode=1` |
| H.264 Profile | `profile-level-id=42801f` |
| 主动呼出 | 纯音频，`VIDMODE_OFF` |
| 接听视频来电 | 音频 `sendrecv`，视频 `recvonly` |

典型 SDP：

```text
v=0
o=- <session-id> <version> IN IP4 <device-ip>
s=-
c=IN IP4 <device-ip>
t=0 0

m=audio <audio-port> RTP/AVP 0 8
a=rtpmap:0 PCMU/8000
a=rtpmap:8 PCMA/8000
a=sendrecv
a=ptime:20
a=rtcp-mux

m=video <video-port> RTP/AVP 96
a=rtpmap:96 H264/90000
a=fmtp:96 packetization-mode=1;profile-level-id=42801f
a=recvonly
a=rtcp-mux
```

连接关系：

```text
<device-ip>:<audio-port>
    └── G.711 音频 RTP/RTCP

<device-ip>:<video-port>
    └── H.264 视频 RTP/RTCP
```

这是一份根据当前代码能力重建的典型 SDP，不是实际成功通话抓包。字段排列、动态 PT、端口和附加属性应以设备运行时抓包为准。

---

## 36. 仓库历史 SDP 逐行解析

仓库 `docs/待处理.md` 中保存了一段历史 SDP：

```text
v=0
o=1006 2588 767 IN IP4 192.168.17.236
s=Talk
c=IN IP4 192.168.17.236
t=0 0
a=rtcp-xr:rcvr-rtt=all:10000 stat-summary=loss,dup,jitt,TTL voip-metrics
a=record:off
m=audio 40481 RTP/AVP 96 0 8 101 97
a=rtpmap:96 opus/48000/2
a=fmtp:96 useinbandfec=1
a=rtpmap:101 telephone-event/48000
a=rtpmap:97 telephone-event/8000
a=rtcp:45664
a=rtcp-fb:* trr-int 1000
a=rtcp-fb:* ccm tmmbr
```

逐行含义：

| SDP | 含义 |
| --- | --- |
| `v=0` | SDP 版本 0 |
| `o=1006 2588 767 IN IP4 192.168.17.236` | 用户 1006 创建的会话，session ID=2588，version=767 |
| `s=Talk` | 会话名称为 Talk |
| `c=IN IP4 192.168.17.236` | 媒体连接地址为 `192.168.17.236` |
| `t=0 0` | 不限定固定开始和结束时间 |
| `a=rtcp-xr:...` | 支持扩展 RTCP 质量报告 |
| `a=record:off` | 对端声明录音关闭，属于客户端扩展属性 |
| `m=audio 40481 RTP/AVP 96 0 8 101 97` | 音频 RTP 端口 40481，提供五个 PT |
| `a=rtpmap:96 opus/48000/2` | PT 96 是双声道 Opus |
| `a=fmtp:96 useinbandfec=1` | Opus 启用带内 FEC |
| `a=rtpmap:101 telephone-event/48000` | PT 101 是 48 kHz DTMF event |
| `a=rtpmap:97 telephone-event/8000` | PT 97 是 8 kHz DTMF event |
| `a=rtcp:45664` | RTCP 使用端口 45664 |
| `a=rtcp-fb:* trr-int 1000` | RTCP 反馈最小间隔相关参数 |
| `a=rtcp-fb:* ccm tmmbr` | 支持临时最大媒体码率反馈 |

该 SDP 没有：

```text
m=video
```

所以它是纯音频 Offer。

该 SDP 也没有：

```text
a=rtcp-mux
```

并且明确给出：

```text
a=rtcp:45664
```

因此该历史抓包中的媒体连接为：

```text
音频 RTP  -> 192.168.17.236:40481
音频 RTCP -> 192.168.17.236:45664
```

该历史呼叫最终返回 `404 Not Found`，没有成功建立媒体，不能用于确认当前版本成功通话的最终 SDP Answer 和 RTP 数据。

---

## 37. SDP 排查清单

### 37.1 检查 Session-level

```text
[ ] v= 是否为 0
[ ] o= 的 session-id/version 是否合理
[ ] c= 是否为可达的媒体 IP
[ ] t= 是否存在
```

### 37.2 检查音频

```text
[ ] 是否存在 m=audio
[ ] 音频端口是否非 0
[ ] 是否包含双方共同支持的 PCMU/PT=0 或 PCMA/PT=8
[ ] a=rtpmap 是否与 m= 中的 PT 一致
[ ] 媒体方向是否允许发送/接收
[ ] ptime 是否合理
[ ] rtcp-mux 或 a=rtcp 是否与实际端口一致
```

### 37.3 检查视频

```text
[ ] 是否存在 m=video
[ ] 视频端口是否非 0
[ ] H.264 PT 是否在 m=video 中
[ ] a=rtpmap 是否为 H264/90000
[ ] packetization-mode 是否匹配
[ ] profile-level-id 是否兼容
[ ] sprop-parameter-sets 如果存在，SPS/PPS 是否有效
[ ] Offer/Answer 的 sendonly/recvonly 是否匹配
```

### 37.4 检查连接

```text
[ ] RTP 目的 IP 是否来自正确的 c=
[ ] RTP 目的端口是否来自对应的 m=
[ ] 是否错误地把 SIP Contact/Via 当成 RTP 地址
[ ] NAT 后 SDP 是否携带不可达私网地址
[ ] 音频和视频是否发送到各自端口
[ ] rtcp-mux 是否双方都协商成功
```

### 37.5 检查抓包

```text
[ ] SIP Offer 与 Answer 是否都存在
[ ] 200 OK 后是否发送 ACK
[ ] SDP 协商完成后是否出现 RTP
[ ] RTP PT 是否能在 SDP 中找到映射
[ ] RTP 目的 IP/端口是否与 SDP 一致
[ ] Sequence 是否连续
[ ] H.264 FU-A 是否有完整 Start/Middle/End
```

---

## 38. 抓包命令

只抓 SIP：

```bash
tcpdump -ni any -s0 -vvv -A 'udp port 5060'
```

抓当前项目 SIP 和 RTP 端口范围：

```bash
tcpdump -ni any -s0 -w /tmp/sip_sdp_rtp.pcap 'udp port 5060 or udp portrange 10000-10100'
```

Wireshark 过滤器：

```text
sip
sdp
rtp
rtcp
sip || rtp || rtcp
```

如果 Wireshark 没有自动识别动态 RTP 端口，可以根据 SDP 中的 `m=audio`、`m=video` 端口执行 `Decode As -> RTP`。

---

## 39. 相关协议

- SDP 基础规范：[RFC 8866](https://datatracker.ietf.org/doc/html/rfc8866)
- SIP Offer/Answer 模型：[RFC 3264](https://datatracker.ietf.org/doc/html/rfc3264)
- RTP 协议：[RFC 3550](https://datatracker.ietf.org/doc/html/rfc3550)
- RTP Audio/Video Profile：[RFC 3551](https://datatracker.ietf.org/doc/html/rfc3551)
- H.264 RTP Payload：[RFC 6184](https://datatracker.ietf.org/doc/html/rfc6184)
- RTP/RTCP 端口复用：[RFC 5761](https://datatracker.ietf.org/doc/html/rfc5761)
- RTP DTMF Event：[RFC 4733](https://datatracker.ietf.org/doc/html/rfc4733)

---

## 40. 核心结论

```text
SDP 是媒体会话说明，不传输音视频数据。

SIP 通过 SDP 协商：
    - 媒体 IP
    - RTP/RTCP 端口
    - codec
    - Payload Type
    - 媒体方向
    - codec 参数

协商完成后：
    - 音频通过独立 G.711 RTP 流传输
    - 视频通过独立 H.264 RTP 流传输
```

最重要的连接判断公式：

```text
RTP 目标地址 = 当前媒体生效的 SDP c= 地址 + SDP m= 端口
```

当前项目典型媒体结构：

```text
一次 SIP 音视频通话
 |
 +-- SIP + SDP：建立会话和协商媒体
 |
 +-- 音频 RTP/RTCP：G.711，sendrecv
 |
 +-- 视频 RTP/RTCP：H.264，本机 recvonly
```
