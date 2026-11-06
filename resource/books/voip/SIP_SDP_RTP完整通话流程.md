# SIP、SDP、RTP 完整通话流程

## 1. 文档目的

本文根据当前项目的 Baresip 2.9.0 集成、账号配置、通话控制和音视频适配实现，梳理一次完整通话中 SIP、SDP、RTP 的关系、报文结构和处理流程。

本文覆盖：

1. Cloud 账号注册；
2. LAN 直拨与 Cloud 呼叫的差异；
3. 主动呼出纯音频通话；
4. 接听音视频来电；
5. SIP INVITE、响应、ACK、BYE 的报文结构；
6. SDP 音视频协商结构；
7. G.711 音频 RTP 报文；
8. H.264 Single NAL、STAP-A、FU-A RTP 报文；
9. 通话取消、忙线和正常挂断流程。

说明：仓库中没有一份当前版本“成功音视频通话”的完整抓包。本文的固定参数来自当前代码；`branch`、`tag`、`Call-ID`、SDP session ID、动态 Payload Type、RTP 端口、Sequence、Timestamp 和 SSRC 等字段为运行时生成，文中的具体值仅用于说明结构。

---

## 2. SIP、SDP、RTP 的关系

```text
SIP：建立、控制和结束通话
 |
 +-- REGISTER：向 SIP 服务器注册
 +-- INVITE：发起通话
 +-- 180 Ringing：振铃
 +-- 200 OK：接听
 +-- ACK：确认建立
 +-- BYE：结束通话
 |
 +-- SIP 消息体携带 SDP
        |
        +-- 协商媒体 IP、端口、编码和方向
        |
        +-- 协商完成后通过 RTP 传输媒体
                |
                +-- 音频：G.711 PCMA / PCMU
                +-- 视频：H.264
                +-- RTCP：RTP 质量统计和控制
```

### 2.1 SDP 是否只是 SIP 的 Payload

在当前项目的 SIP 通话场景中，可以把 SDP 理解为 SIP 的 Payload。更准确的协议术语是：

> SDP 是 SIP 消息可携带的一种 Message Body（消息体），其 MIME 类型为 `application/sdp`。

报文封装关系如下：

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

一个带 SDP 的 SIP INVITE 示例：

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

空行之前是 SIP 起始行和 Headers；空行之后以 `v=0` 开始的内容是 SDP Body。`Content-Type: application/sdp` 告诉接收方如何解析消息体，`Content-Length` 表示消息体的字节长度。

### 2.2 SIP 和 SDP 的职责边界

SIP 负责控制通话：

```text
REGISTER    注册账号
INVITE      发起通话
180         对方振铃
200 OK      对方接听
ACK         确认建立
CANCEL      取消尚未建立的呼叫
BYE         结束已经建立的通话
```

SDP 负责描述媒体如何传输：

```text
我的 RTP IP 是什么？
我的 RTP 端口是多少？
我支持哪些 codec？
Payload Type 如何映射到 codec？
媒体方向是 sendrecv、sendonly 还是 recvonly？
H.264 使用哪种 RTP 分包模式？
```

例如：

```text
c=IN IP4 192.168.1.10
```

表示媒体连接地址。

```text
m=audio 10000 RTP/AVP 0 8
```

表示音频 RTP 端口为 10000，支持 PT 0 和 PT 8。

```text
a=rtpmap:0 PCMU/8000
a=rtpmap:8 PCMA/8000
```

表示 PT 0 对应 PCMU/8000，PT 8 对应 PCMA/8000。

```text
a=sendrecv
```

表示该路媒体允许双向发送和接收。

### 2.3 SDP 不传输音视频数据

SDP 只描述媒体参数，不承载实际声音和视频。Offer/Answer 协商完成后，RTP 使用 SDP 中协商出的 IP、端口和 codec，通过独立的 UDP 报文传输媒体：

```text
建立阶段：

UDP 5060
└── SIP
    └── SDP：协商 RTP 怎么传


通话阶段：

UDP <audio-port>
└── RTP
    └── G.711 音频 Payload

UDP <video-port>
└── RTP
    └── H.264 视频 Payload
```

因此下面这种理解是错误的：

```text
SIP
└── SDP
    └── RTP
        └── 音视频数据
```

正确关系是：

```text
SIP + SDP：建立会话并协商媒体参数

RTP：根据协商结果，独立传输音视频数据
```

对应完整时序：

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

### 2.4 并非所有 SIP 消息都携带 SDP

| SIP 消息                | 通常是否携带 SDP | 说明                           |
| ----------------------- | ---------------- | ------------------------------ |
| `REGISTER`              | 否               | 只注册 SIP 联系地址            |
| `INVITE`                | 通常是           | 常作为 SDP Offer               |
| `100 Trying`            | 否               | 临时信令响应                   |
| `180 Ringing`           | 通常否           | 也允许携带 SDP                 |
| `183 Session Progress`  | 经常是           | 常用于早期媒体 SDP             |
| `200 OK`（响应 INVITE） | 通常是           | 常作为 SDP Answer              |
| `ACK`                   | 通常否           | Delayed Offer 场景可能携带 SDP |
| `BYE`                   | 通常否           | 结束已建立的对话               |
| `200 OK`（响应 BYE）    | 否               | 确认通话结束                   |

SDP 从协议上并不专属于 SIP，也可以由 HTTP、邮件或其他协议传输；但在当前项目中，SDP 就是作为 SIP 消息体参与 Offer/Answer 协商。

### 2.5 当前项目事件上的区别

- `UA_EVENT_CALL_REMOTE_SDP`：已经收到并解析了远端 SDP，知道对方声明的媒体能力；
- `UA_EVENT_CALL_ESTABLISHED`：SIP 对话已经建立；
- `UA_EVENT_CALL_RTPESTAB`：某一路音频或视频 RTP 已经实际开始传输；
- SIP 对话建立不保证 RTP 已经到达；
- 存在早期媒体时，RTP 可能在最终 `200 OK` 和 `ACK` 前开始传输。

### 2.6 音频和视频是否通过同一个 RTP 通道传输

当前项目中，音频和视频是两路独立的 RTP 媒体流，通常使用不同的 UDP 端口传输。H.264 是纯视频编码格式，本身不包含音频。

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

当前项目从 `10000~10100` 中动态分配 RTP 端口。以下端口只用于说明结构：

```text
音频：UDP 10000 -> RTP -> G.711 PCMA/PCMU
视频：UDP 10002 -> RTP -> H.264
```

SDP 使用两个独立的 `m=` 段描述这两路媒体：

```text
m=audio 10000 RTP/AVP 8
a=rtpmap:8 PCMA/8000
a=sendrecv
a=rtcp-mux

m=video 10002 RTP/AVP 96
a=rtpmap:96 H264/90000
a=recvonly
a=rtcp-mux
```

含义为：

```text
m=audio：
    媒体类型 = 音频
    本地接收端口 = 10000
    PT = 8
    codec = PCMA

m=video：
    媒体类型 = 视频
    本地接收端口 = 10002
    PT = 96（示例动态 PT）
    codec = H.264
```

远端发送到本机时的典型通道：

```text
远端设备
 |
 +-- G.711 音频 RTP -----> 本机 UDP 10000
 |
 +-- H.264 视频 RTP -----> 本机 UDP 10002
```

音频 RTP 报文只携带音频：

```text
+-------------------------+
| IP Header               |
+-------------------------+
| UDP Header              |
+-------------------------+
| RTP Header              |
| PT = 0 或 8             |
+-------------------------+
| G.711 Payload           |
| PCMU 或 PCMA 音频数据   |
+-------------------------+
```

视频 RTP 报文只携带视频：

```text
+-------------------------+
| IP Header               |
+-------------------------+
| UDP Header              |
+-------------------------+
| RTP Header              |
| PT = SDP 动态协商值     |
+-------------------------+
| H.264 Payload           |
| Single NAL/STAP-A/FU-A  |
+-------------------------+
```

#### 2.6.1 H.264 为什么不包含音频

H.264 是视频编码标准，只编码图像帧：

```text
图像帧
 |
 v
H.264 Encoder
 |
 v
SPS / PPS / IDR / P-Slice 等 NAL Unit
```

H.264 数据可以包含 SPS、PPS、IDR、非 IDR Slice、SEI 等视频数据，但不包含 G.711、AAC、Opus 或 PCM 音频。

下面这种理解是错误的：

```text
H.264
├── 视频
└── 音频
```

正确关系是：

```text
视频编码：H.264
音频编码：G.711
```

MP4、MKV、MPEG-TS 等容器可以同时封装音频轨和视频轨，所以一个视频文件通常既有画面也有声音。例如：

```text
MP4 容器
 |
 +-- Video Track
 |      └── H.264
 |
 +-- Audio Track
        └── AAC
```

这里是 MP4 容器同时包含 H.264 视频轨和 AAC 音频轨，不是 H.264 包含 AAC。当前项目不通过 MP4 或 MPEG-TS 承载 SIP 音视频，而是分别发送 G.711 RTP 和 H.264 RTP。

#### 2.6.2 `rtcp_mux` 不会合并音频和视频

当前项目启用：

```c
config->avt.rtcp_mux = true;
```

`rtcp_mux` 表示同一路媒体的 RTP 和 RTCP 可以复用同一个 UDP 端口：

```text
UDP 10000：
    音频 RTP
    音频 RTCP

UDP 10002：
    视频 RTP
    视频 RTCP
```

它不表示把音频 RTP 和视频 RTP 合并到同一个通道。当前项目的典型结构仍然是：

```text
              +-- UDP 10000：音频 RTP + 音频 RTCP
SIP 通话 -----|
              +-- UDP 10002：视频 RTP + 视频 RTCP
```

#### 2.6.3 当前项目视频来电的数据方向

接听视频来电时，当前设备设置：

```text
音频：sendrecv
视频：recvonly
```

因此实际传输方向为：

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

对应设备功能：

```text
本机麦克风  ---- G.711 RTP ----> 远端扬声器
本机扬声器  <--- G.711 RTP ----- 远端麦克风
本机显示屏  <--- H.264 RTP ----- 远端摄像头
```

#### 2.6.4 两路 RTP 如何关联和同步

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

音频和视频 Timestamp 不能直接进行数值比较，因为音频时钟通常是 8 kHz，H.264 视频时钟是 90 kHz。标准 RTP 系统一般通过 RTCP Sender Report 中的 NTP 时间与 RTP Timestamp 映射，将两路媒体关联到共同时间轴：

```text
音频 RTCP SR：NTP 时间 <-> 音频 RTP Timestamp
视频 RTCP SR：NTP 时间 <-> 视频 RTP Timestamp
```

当前项目的自定义 H.264 硬件重组器主要使用 Sequence 和 Marker 重组 Access Unit，没有在该 decoder 层使用 RTP Timestamp 做音视频同步。

一句话总结：

```text
一次音视频通话
 |
 +-- SIP 信令
 |
 +-- 音频 RTP：G.711
 |
 +-- 视频 RTP：H.264
```

H.264 只包含视频，不包含音频；当前项目的音频和视频是两路独立 RTP 媒体流。

---

## 3. 当前项目确定的协议参数

| 项目             | 当前实现                                                 |
| ---------------- | -------------------------------------------------------- |
| SIP 协议栈       | Baresip 2.9.0 / libre                                    |
| SIP 传输         | UDP                                                      |
| SIP 本地监听     | `0.0.0.0:5060`                                           |
| LAN 账号         | Direct/P2P，`regint=0`，不发送 `REGISTER`                |
| Cloud 账号       | Registrar 模式，需要注册                                 |
| Cloud 注册有效期 | 配置为 30 秒，由 Baresip 自动续租                        |
| 最大并发通话     | 1 路                                                     |
| 呼叫超时         | 30 秒                                                    |
| RTP 本地端口范围 | `10000~10100`                                            |
| RTP 接收超时     | 60 秒                                                    |
| RTP 接收带宽上限 | 512 Kbit/s                                               |
| RTP/RTCP         | 启用 `rtcp_mux`                                          |
| 音频 codec       | G.711 PCMU/PT=0、PCMA/PT=8                               |
| 音频 RTP 时钟    | 8000 Hz                                                  |
| 本机音频硬件格式 | 16000 Hz、mono、S16LE                                    |
| 视频 codec       | H.264/90000                                              |
| H.264 分包模式   | 主要使用 `packetization-mode=1`，同时注册 mode 0 decoder |
| H.264 Profile    | Baseline Level 3.1，`profile-level-id=42801f`            |
| 本机主动呼出     | 仅音频，`VIDMODE_OFF`                                    |
| 本机接听视频来电 | 音频 `sendrecv`，视频 `recvonly`                         |

关键实现文件：

- `src/business/intercom/signaling/sip_runtime.c`
- `src/business/intercom/signaling/sip_account.c`
- `src/business/intercom/signaling/sip_session.c`
- `src/business/intercom/signaling/sip_session_baresip_event.c`
- `src/business/intercom/signaling/adapter_audio.c`
- `src/business/intercom/signaling/adapter_video_codec.c`
- `docs/adapter.md`

---

## 4. 网络报文整体封装

### 4.1 SIP 报文

```text
+------------------------------+
| Ethernet Header（可选）      |
+------------------------------+
| IPv4 / IPv6 Header           |
+------------------------------+
| UDP Header                   |
+------------------------------+
| SIP Request / Response       |  ASCII 文本
|                              |
| SIP Headers                  |
|                              |
| 空行                         |
|                              |
| SDP Body（可选）             |
+------------------------------+
```

### 4.2 RTP 报文

```text
+------------------------------+
| Ethernet Header（可选）      |
+------------------------------+
| IPv4 / IPv6 Header           |
+------------------------------+
| UDP Header                   |
+------------------------------+
| RTP Header，至少 12 字节     |
+------------------------------+
| RTP Payload                  |
| G.711 或 H.264               |
+------------------------------+
| Padding（可选）              |
+------------------------------+
```

SIP 是 ASCII 文本协议，RTP 是二进制协议。SIP 和 RTP 使用不同 UDP 端口：

```text
SIP：本机 UDP 5060
RTP：本机从 10000~10100 中动态分配
```

---

## 5. Cloud 账号注册流程

LAN 账号设置 `regint=0`，创建 UA 后直接可用，不执行以下注册流程。Cloud 账号使用 Registrar 模式。

### 5.1 注册时序图

```text
设备/Baresip                    SIP Registrar
     |                                |
     |  REGISTER（第一次不带认证）    |
     |------------------------------->|
     |                                |
     |  401 Unauthorized              |
     |  WWW-Authenticate: Digest ...  |
     |<-------------------------------|
     |                                |
     |  REGISTER                      |
     |  Authorization: Digest ...     |
     |------------------------------->|
     |                                |
     |  200 OK                        |
     |  Contact / Expires             |
     |<-------------------------------|
     |                                |
     |====== 账号注册成功 ============|
     |                                |
     |  到期前自动发送 REGISTER 续租  |
     |------------------------------->|
     |                                |
     |  200 OK                        |
     |<-------------------------------|
```

如果服务器不要求认证，也可能在第一次 `REGISTER` 后直接返回 `200 OK`。

### 5.2 第一次 REGISTER

以下字段由 Baresip 在运行时生成：

```text
REGISTER sip:<registrar-host> SIP/2.0
Via: SIP/2.0/UDP <local-ip>:5060;branch=z9hG4bK...;rport
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

### 5.3 401 Digest 认证挑战

```text
SIP/2.0 401 Unauthorized
Via: SIP/2.0/UDP <local-ip>:5060;branch=z9hG4bK...
From: <sip:<username>@<domain>>;tag=<from-tag>
To: <sip:<username>@<domain>>;tag=<server-tag>
Call-ID: <same-register-call-id>
CSeq: 1 REGISTER
WWW-Authenticate: Digest realm="...", nonce="...", algorithm=MD5
Content-Length: 0
```

### 5.4 携带认证重新 REGISTER

```text
REGISTER sip:<registrar-host> SIP/2.0
Via: SIP/2.0/UDP <local-ip>:5060;branch=z9hG4bK...;rport
From: <sip:<username>@<domain>>;tag=<from-tag>
To: <sip:<username>@<domain>>
Call-ID: <same-register-call-id>
CSeq: 2 REGISTER
Contact: <sip:<username>@<local-ip>:5060;transport=udp>
Expires: 30
Authorization: Digest username="...",
               realm="...",
               nonce="...",
               uri="sip:<registrar-host>",
               response="..."
Content-Length: 0
```

注册成功：

```text
SIP/2.0 200 OK
Via: ...
From: ...
To: ...;tag=<server-tag>
Call-ID: <same-register-call-id>
CSeq: 2 REGISTER
Contact: <sip:<username>@<local-ip>:5060;transport=udp>;expires=30
Content-Length: 0
```

---

## 6. 主动呼出：完整纯音频通话

当前项目通过以下调用发起主动呼叫：

```c
ua_connect(account->ua, &call, NULL, uri, VIDMODE_OFF);
```

因此主动呼出固定为：

```text
音频：sendrecv
视频：off
```

目标地址生成规则：

```text
LAN 直拨：sip:<callee>@<LAN-IP>
Cloud：   sip:<callee>@<domain>
完整 URI：业务传入 sip: URI 时原样使用
```

### 6.1 Cloud/代理呼出时序

```text
本设备/Baresip              SIP Proxy/Registrar              被叫设备
      |                              |                           |
      |  INVITE + SDP Offer          |                           |
      |----------------------------->|                           |
      |                              |  INVITE + SDP Offer       |
      |                              |-------------------------->|
      |                              |                           |
      |  100 Trying                  |                           |
      |<-----------------------------|                           |
      |                              |                           |
      |                              |  180 Ringing              |
      |                              |<--------------------------|
      |  180 Ringing                 |                           |
      |<-----------------------------|                           |
      |                              |                           |
      |      本项目进入 RINGING 状态                             |
      |                              |                           |
      |                              |  200 OK + SDP Answer      |
      |                              |<--------------------------|
      |  200 OK + SDP Answer         |                           |
      |<-----------------------------|                           |
      |                              |                           |
      |  ACK                         |                           |
      |----------------------------->|  ACK                      |
      |                              |-------------------------->|
      |                              |                           |
      |========== SIP 对话建立：CALL_ESTABLISHED ================|
      |                              |                           |
      |<========= G.711 RTP 音频双向传输 ========================>|
      |<========= RTCP（与 RTP 复用端口）========================>|
      |                              |                           |
      |========== 收到首个 RTP：CALL_RTPESTAB("audio") =========|
      |                              |                           |
      |  BYE                         |                           |
      |----------------------------->|  BYE                      |
      |                              |-------------------------->|
      |                              |                           |
      |                              |  200 OK                   |
      |                              |<--------------------------|
      |  200 OK                      |                           |
      |<-----------------------------|                           |
      |                              |                           |
      |==================== 通话结束 =============================|
```

### 6.2 LAN 直拨时序

LAN Direct 模式不经过 SIP Registrar/Proxy：

```text
本设备/Baresip                              对端设备
      |                                         |
      |  INVITE + SDP Offer                     |
      |---------------------------------------->|
      |  100 Trying / 180 Ringing               |
      |<----------------------------------------|
      |  200 OK + SDP Answer                    |
      |<----------------------------------------|
      |  ACK                                    |
      |---------------------------------------->|
      |                                         |
      |<========== 双向 G.711 RTP =============>|
      |                                         |
      |  BYE                                    |
      |---------------------------------------->|
      |  200 OK                                 |
      |<----------------------------------------|
```

---

## 7. SIP INVITE 报文结构

### 7.1 INVITE + 音频 SDP Offer

下面是按当前项目能力重建的结构示例。实际 codec 顺序、扩展属性和 RTP 端口以抓包为准。

```text
INVITE sip:<callee>@<target-host> SIP/2.0
Via: SIP/2.0/UDP <local-ip>:5060;branch=z9hG4bK...;rport
Max-Forwards: 70
From: <sip:<local-user>@<local-domain>>;tag=<local-tag>
To: <sip:<callee>@<target-host>>
Call-ID: <call-id>
CSeq: 1 INVITE
Contact: <sip:<local-user>@<local-ip>:5060;transport=udp>
Allow: INVITE, ACK, BYE, CANCEL, OPTIONS, INFO, REFER, NOTIFY
Supported: ...
Content-Type: application/sdp
Content-Length: <SDP长度>

v=0
o=- <session-id> <session-version> IN IP4 <local-ip>
s=-
c=IN IP4 <local-ip>
t=0 0
m=audio <local-audio-rtp-port> RTP/AVP 0 8
a=rtpmap:0 PCMU/8000
a=rtpmap:8 PCMA/8000
a=sendrecv
a=rtcp-mux
a=ptime:20
```

注意：

1. `a=ptime:20` 是常见值，最终是否出现以及具体值由 Baresip 协商决定；
2. PCMU 和 PCMA 的排列顺序可能不同；
3. PT 0/8 是静态 Payload Type，`a=rtpmap` 可以省略；
4. Baresip 可能附加 `telephone-event`、RTCP feedback 或其他属性；
5. 当前主动呼出使用 `VIDMODE_OFF`，不会建立视频流。

### 7.2 100 Trying

```text
SIP/2.0 100 Trying
Via: SIP/2.0/UDP <local-ip>:5060;branch=z9hG4bK...
From: <sip:<local-user>@<local-domain>>;tag=<local-tag>
To: <sip:<callee>@<target-host>>
Call-ID: <same-call-id>
CSeq: 1 INVITE
Content-Length: 0
```

### 7.3 180 Ringing

```text
SIP/2.0 180 Ringing
Via: SIP/2.0/UDP <local-ip>:5060;branch=z9hG4bK...
From: <sip:<local-user>@<local-domain>>;tag=<local-tag>
To: <sip:<callee>@<target-host>>;tag=<remote-tag>
Call-ID: <same-call-id>
CSeq: 1 INVITE
Contact: <sip:<callee>@<remote-ip>:5060>
Content-Length: 0
```

对应项目事件：

```text
UA_EVENT_CALL_RINGING
    |
    +--> INTERCOM_STATUS_RINGING
```

### 7.4 200 OK + SDP Answer

假设对端选择 PCMA：

```text
SIP/2.0 200 OK
Via: SIP/2.0/UDP <local-ip>:5060;branch=z9hG4bK...
From: <sip:<local-user>@<local-domain>>;tag=<local-tag>
To: <sip:<callee>@<target-host>>;tag=<remote-tag>
Call-ID: <same-call-id>
CSeq: 1 INVITE
Contact: <sip:<callee>@<remote-ip>:5060>
Content-Type: application/sdp
Content-Length: <SDP长度>

v=0
o=- <remote-session-id> <remote-version> IN IP4 <remote-ip>
s=-
c=IN IP4 <remote-ip>
t=0 0
m=audio <remote-audio-rtp-port> RTP/AVP 8
a=rtpmap:8 PCMA/8000
a=sendrecv
a=rtcp-mux
a=ptime:20
```

协商结果：

```text
本机发送音频 RTP：
    源端口 = 本机 SDP m=audio 中的端口
    目的端口 = 对端 SDP m=audio 中的端口
    PT = 8
    codec = PCMA

本机接收音频 RTP：
    对端发送到本机 SDP m=audio 中的端口
```

### 7.5 ACK

```text
ACK sip:<callee>@<remote-ip>:5060 SIP/2.0
Via: SIP/2.0/UDP <local-ip>:5060;branch=z9hG4bK...
From: <sip:<local-user>@<local-domain>>;tag=<local-tag>
To: <sip:<callee>@<target-host>>;tag=<remote-tag>
Call-ID: <same-call-id>
CSeq: 1 ACK
Max-Forwards: 70
Content-Length: 0
```

INVITE 对话建立过程：

```text
INVITE
   |
   v
200 OK
   |
   v
ACK
```

---

## 8. 接听音视频来电

接听视频来电时，项目设置：

```c
call_set_media_direction(
    call,
    SDP_SENDRECV,  // audio
    SDP_RECVONLY   // video
);
```

媒体方向为：

```text
远端：发送音频 + H.264 视频
本机：发送音频，不发送视频
```

### 8.1 完整时序图

```text
远端设备                                      本设备/Baresip
    |                                               |
    |  INVITE                                       |
    |  SDP Offer:                                   |
    |    audio=sendrecv                             |
    |    video=sendonly/sendrecv                    |
    |    H264/90000                                 |
    |---------------------------------------------->|
    |                                               |
    |                    UA_EVENT_CALL_INCOMING     |
    |                    CALL_REMOTE_SDP("offer")  |
    |                                               |
    |  100 Trying                                   |
    |<----------------------------------------------|
    |                                               |
    |  180 Ringing                                  |
    |<----------------------------------------------|
    |                                               |
    |             用户选择视频接听                  |
    |             audio = sendrecv                  |
    |             video = recvonly                  |
    |                                               |
    |  200 OK                                       |
    |  SDP Answer:                                  |
    |    audio=sendrecv                             |
    |    video=recvonly                             |
    |<----------------------------------------------|
    |                                               |
    |  ACK                                          |
    |---------------------------------------------->|
    |                                               |
    |========== SIP 对话建立 =======================|
    |                                               |
    |<========= 本机上行 G.711 音频 RTP ============|
    |========== 远端下行 G.711 音频 RTP ===========>|
    |========== 远端下行 H.264 视频 RTP ===========>|
    |                                               |
    |      本机不会发送 H.264 视频 RTP              |
    |                                               |
    |              CALL_RTPESTAB("audio")          |
    |              CALL_RTPESTAB("video")          |
    |                                               |
    |  BYE                                          |
    |---------------------------------------------->|
    |  200 OK                                       |
    |<----------------------------------------------|
```

### 8.2 远端视频 SDP Offer 示例

```text
v=0
o=- <session-id> <version> IN IP4 <remote-ip>
s=-
c=IN IP4 <remote-ip>
t=0 0

m=audio <remote-audio-port> RTP/AVP 0 8
a=rtpmap:0 PCMU/8000
a=rtpmap:8 PCMA/8000
a=sendrecv
a=rtcp-mux

m=video <remote-video-port> RTP/AVP 96
a=rtpmap:96 H264/90000
a=fmtp:96 packetization-mode=1;profile-level-id=42801f;sprop-parameter-sets=<SPS>,<PPS>
a=sendonly
a=rtcp-mux
```

远端也可能声明：

```text
a=sendrecv
```

本机仍会在 Answer 中将本机视频方向收窄为：

```text
a=recvonly
```

### 8.3 本机 200 OK 中的 SDP Answer

下面端口仅为示例，实际端口从 `10000~10100` 中动态分配：

```text
v=0
o=- <local-session-id> <version> IN IP4 <local-ip>
s=-
c=IN IP4 <local-ip>
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

注意：

- `96` 是示例动态 PT，不能在接收代码中写死；
- 本地 H.264 FMTP 固定声明 `profile-level-id=42801f`；
- 当前 H.264 adapter 只有 decoder，没有 encoder；
- 如果远端 SDP 带 `sprop-parameter-sets`，项目会解析其中的 Base64 SPS/PPS；
- 如果 SDP 不带 SPS/PPS，项目会从后续 RTP 中等待内联 SPS/PPS；
- 如果视频能力不可用，项目会退化为纯音频，不影响音频通话。

---

## 9. SDP 字段说明

### 9.1 会话字段

```text
v=0
```

SDP 协议版本，固定为 0。

```text
o=- 123456 123457 IN IP4 192.168.1.10
```

结构：

```text
o=<username> <session-id> <version> <network> <address-type> <address>
```

其中：

- `session-id`：会话标识；
- `version`：SDP 修改时递增；
- `IN IP4`：Internet/IPv4。

```text
s=-
```

会话名称。

```text
c=IN IP4 192.168.1.10
```

媒体连接地址，即 RTP 发送目标 IP。

```text
t=0 0
```

不设置绝对开始和结束时间。

### 9.2 音频媒体字段

```text
m=audio 10000 RTP/AVP 0 8
```

表示：

```text
媒体类型：audio
RTP 端口：10000
传输协议：RTP/AVP
支持 PT：0、8
```

```text
a=rtpmap:0 PCMU/8000
a=rtpmap:8 PCMA/8000
```

表示 PT 到 codec 的映射。

### 9.3 视频媒体字段

```text
m=video 10002 RTP/AVP 96
a=rtpmap:96 H264/90000
```

表示动态 PT 96 对应 H.264，RTP 时钟为 90 kHz。

```text
a=fmtp:96 packetization-mode=1;profile-level-id=42801f
```

H.264 格式参数：

- `packetization-mode=1`：允许 Single NAL、STAP-A、FU-A；
- `profile-level-id=42801f`：Baseline Level 3.1；
- `sprop-parameter-sets`：可选的 SPS/PPS。

### 9.4 媒体方向

```text
a=sendrecv    双向发送和接收
a=sendonly    只发送
a=recvonly    只接收
a=inactive    不发送也不接收
```

---

## 10. RTP 固定头结构

RTP 固定头最少 12 字节：

```text
  0                   1                   2                   3
  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
 |V=2|P|X|  CC   |M|     PT      |       Sequence Number         |
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
 |                           Timestamp                           |
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
 |                 SSRC - Synchronization Source                 |
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
 |                 CSRC list（CC > 0 时存在）                    |
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
 |                 Extension（X = 1 时存在）                     |
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
 |                 RTP Payload                                  |
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

| 字段        | 位数 | 含义                              |
| ----------- | ---: | --------------------------------- |
| `V`         |    2 | RTP 版本，当前为 2                |
| `P`         |    1 | 是否存在尾部 Padding              |
| `X`         |    1 | 是否存在扩展头                    |
| `CC`        |    4 | CSRC 数量                         |
| `M`         |    1 | Marker，含义由 codec 定义         |
| `PT`        |    7 | Payload Type，通过 SDP 映射 codec |
| `Sequence`  |   16 | 每发送一个 RTP 包递增 1           |
| `Timestamp` |   32 | 媒体采样时钟，不是 Unix 时间      |
| `SSRC`      |   32 | RTP 同步源标识                    |

---

## 11. G.711 音频 RTP

### 11.1 典型 20 ms 报文

G.711 参数：

```text
采样率 = 8000 sample/s
包时长 = 20 ms
Payload = 8000 * 0.020 = 160 字节
```

```text
+----------------------------+-------------------------------+
| RTP Header                 | G.711 Payload                 |
| 通常 12 字节               | 典型 160 字节                 |
+----------------------------+-------------------------------+
```

如果不考虑 IP/UDP 选项：

```text
IPv4 Header       20 字节
UDP Header         8 字节
RTP Header        12 字节
G.711 Payload    160 字节
--------------------------------
IP Packet        200 字节
```

加上普通以太网头后约为 214 字节，不含 VLAN 和 FCS 等额外字段。

### 11.2 PCMA 示例

```text
RTP Header:
    V         = 2
    PT        = 8          -> PCMA
    Sequence  = 32001
    Timestamp = 160000
    SSRC      = 0x12345678

Payload:
    160 字节 G.711 A-law 数据
```

下一个 20 ms RTP 包通常为：

```text
Sequence  = 32002
Timestamp = 160160
```

即：

```text
Sequence 每个 RTP 包 +1
Timestamp 每 20 ms +160
```

### 11.3 PCMU 示例

```text
PT = 0 -> PCMU
```

其他结构与 PCMA 相同。

### 11.4 项目音频上行链路

```text
麦克风
 |
 v
HAL audio input
 |
 v
hdl_audio source
 |
 v
16 kHz / mono / S16LE
 |
 v
auresamp -> 8 kHz
 |
 v
G.711 encoder
 |
 v
RTP Header + G.711 Payload
 |
 v
UDP
 |
 v
远端
```

### 11.5 项目音频下行链路

```text
网卡
 |
 v
UDP
 |
 v
libre RTP 解析
 |
 +-- 解析 PT / Sequence / Timestamp / SSRC
 |
 v
Baresip jitter buffer
 |
 v
G.711 decoder
 |
 v
8 kHz PCM
 |
 v
auresamp
 |
 v
16 kHz / mono / S16LE
 |
 v
hdl_audio player
 |
 v
HAL audio output
 |
 v
扬声器
```

---

## 12. H.264 视频 RTP

H.264 的 RTP PT 是 SDP 动态协商值，例如 96，不能写死。

```text
PT        = SDP 中 H264 对应的 PT
Timestamp = 90 kHz 时钟
Marker    = 通常表示 Access Unit 的最后一个 RTP 包
```

同一个视频 Access Unit 被拆成多个 RTP 包时：

```text
Sequence：每个 RTP 包递增
Timestamp：通常保持相同
Marker：最后一个 RTP 包通常设置为 1
```

### 12.1 H.264 NAL Header

```text
  7   6 5   4 3 2 1 0
 +---+-----+-----------+
 | F | NRI | NAL Type  |
 +---+-----+-----------+
```

常见 NAL Type：

| NAL Type | 含义         |
| -------: | ------------ |
|        1 | 非 IDR Slice |
|        5 | IDR Slice    |
|        7 | SPS          |
|        8 | PPS          |
|       24 | STAP-A       |
|       28 | FU-A         |

### 12.2 Single NAL Unit

适用于一个完整 NAL 能放入一个 RTP 包的情况：

```text
+------------------------+
| RTP Header             |
+------------------------+
| H.264 NAL Header       |  1 字节
+------------------------+
| NAL Payload            |
+------------------------+
```

### 12.3 STAP-A

STAP-A 把多个较小 NAL 聚合进一个 RTP Payload：

```text
+------------------------+
| STAP-A Header，1 字节  |  NAL Type = 24
+------------------------+
| NALU-1 Size，2 字节    |
+------------------------+
| NALU-1                 |
+------------------------+
| NALU-2 Size，2 字节    |
+------------------------+
| NALU-2                 |
+------------------------+
| ...                    |
+------------------------+
```

例如：

```text
一个 RTP Payload
    |
    +-- STAP-A Header
    +-- SPS 长度
    +-- SPS
    +-- PPS 长度
    +-- PPS
```

STAP-A 属于 `packetization-mode=1`。

### 12.4 FU-A

当一个 H.264 NAL 太大，无法放入一个 RTP 包时，使用 FU-A 分片：

```text
原始大 NAL
    |
    +--> RTP 1：FU-A Start
    +--> RTP 2：FU-A Middle
    +--> RTP 3：FU-A Middle
    +--> RTP 4：FU-A End
```

FU-A Payload：

```text
+-------------------------+
| FU Indicator            |  1 字节
+-------------------------+
| FU Header               |  1 字节
+-------------------------+
| NAL Fragment            |
+-------------------------+
```

FU Indicator：

```text
  7   6 5   4 3 2 1 0
 +---+-----+-----------+
 | F | NRI | Type=28   |
 +---+-----+-----------+
```

FU Header：

```text
  7   6   5   4 3 2 1 0
 +---+---+---+-----------+
 | S | E | R | NAL Type  |
 +---+---+---+-----------+
```

| 字段       | 含义           |
| ---------- | -------------- |
| `S=1`      | 第一个分片     |
| `E=1`      | 最后一个分片   |
| `R`        | 保留，必须为 0 |
| `NAL Type` | 原始 NAL Type  |

FU-A 分包示例：

```text
RTP #100:
    Sequence  = 100
    Timestamp = 900000
    Marker    = 0
    S=1 E=0
    Payload   = NAL 前半部分

RTP #101:
    Sequence  = 101
    Timestamp = 900000
    Marker    = 0
    S=0 E=0
    Payload   = NAL 中间部分

RTP #102:
    Sequence  = 102
    Timestamp = 900000
    Marker    = 1
    S=0 E=1
    Payload   = NAL 最后部分
```

项目重组后输出 Annex-B：

```text
00 00 00 01
原始 NAL Header
完整 NAL Payload
```

当前自定义硬件 decoder 回调拿到：

```text
Marker
Sequence
H.264 RTP Payload
```

RTP Timestamp 已由 Baresip/libre 解析，但当前项目硬件重组器主要使用 Sequence 和 Marker 判断连续性及 Access Unit 边界。

### 12.5 项目视频下行链路

```text
远端摄像头
 |
 v
H.264 Encoder
 |
 v
Single NAL / STAP-A / FU-A
 |
 v
RTP/UDP
 |
 v
libre 解析 RTP Header
 |
 v
Baresip H.264 decoder callback
 |
 | 传入：Marker + Sequence + RTP Payload
 v
项目 H.264 depacketizer
 |
 v
Annex-B Access Unit
 |
 v
等待 SPS/PPS 和 IDR
 |
 v
SSD20x VDEC / x86 FFmpeg
 |
 v
视频显示
```

---

## 13. 通话结束报文

### 13.1 已建立通话后挂断

挂断方发送：

```text
BYE sip:<peer-contact> SIP/2.0
Via: SIP/2.0/UDP <local-ip>:5060;branch=z9hG4bK...
From: <sip:...>;tag=<local-tag>
To: <sip:...>;tag=<remote-tag>
Call-ID: <same-call-id>
CSeq: 2 BYE
Max-Forwards: 70
Content-Length: 0
```

对端响应：

```text
SIP/2.0 200 OK
Via: ...
From: ...
To: ...
Call-ID: <same-call-id>
CSeq: 2 BYE
Content-Length: 0
```

项目收到 `UA_EVENT_CALL_CLOSED` 后：

```text
UA_EVENT_CALL_CLOSED
        |
        v
清理 active_session
        |
        +--> 停止视频 receiver / decoder / display
        |
        +--> 停止 Baresip 音频 source/player
        |
        +--> 关闭功放
        |
        +--> 恢复系统普通音频管理
        |
        +--> 发布 INTERCOM_STATUS_ENDED
```

### 13.2 振铃阶段取消

如果 INVITE 尚未收到最终响应，主叫使用 CANCEL：

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

### 13.3 忙线

当前项目最多只允许一路通话。已有活动通话时，新来电会被拒绝：

```text
SIP/2.0 486 Busy Here
```

事务结束流程：

```text
INVITE
   |
   v
486 Busy Here
   |
   v
ACK
```

---

## 14. 一次完整视频来电总流程

```text
+----------------------------------------------------------------+
| 1. Cloud 账号 REGISTER                                         |
|    REGISTER -> 401 -> REGISTER + Authorization -> 200 OK        |
+-------------------------------+--------------------------------+
                                |
                                v
+----------------------------------------------------------------+
| 2. 收到 INVITE + SDP Offer                                     |
|                                                                |
|    audio: PCMU / PCMA                                          |
|    video: H264/90000, packetization-mode=1                     |
+-------------------------------+--------------------------------+
                                |
                                v
+----------------------------------------------------------------+
| 3. 项目检查                                                     |
|                                                                |
|    - 是否允许室内通话                                          |
|    - 是否已有活动通话                                          |
|    - 是否支持 H.264 decoder                                    |
|    - UI 是否启用可视对讲                                       |
|    - SDP 是否包含 video                                        |
+-------------------------------+--------------------------------+
                                |
                                v
+----------------------------------------------------------------+
| 4. 振铃                                                         |
|                                                                |
|    SIP: 180 Ringing                                            |
|    Business: INTERCOM_STATUS_INCOMING                          |
+-------------------------------+--------------------------------+
                                |
                                v
+----------------------------------------------------------------+
| 5. 用户接听                                                     |
|                                                                |
|    audio = sendrecv                                            |
|    video = recvonly                                            |
|    发送 200 OK + SDP Answer                                    |
+-------------------------------+--------------------------------+
                                |
                                v
+----------------------------------------------------------------+
| 6. 收到 ACK                                                     |
|                                                                |
|    UA_EVENT_CALL_ESTABLISHED                                   |
+-------------------------------+--------------------------------+
                                |
                                v
+----------------------------------------------------------------+
| 7. RTP 传输                                                     |
|                                                                |
|    本机 -> 远端：G.711 音频                                    |
|    远端 -> 本机：G.711 音频                                    |
|    远端 -> 本机：H.264 视频                                    |
|                                                                |
|    UA_EVENT_CALL_RTPESTAB("audio")                             |
|    UA_EVENT_CALL_RTPESTAB("video")                             |
+-------------------------------+--------------------------------+
                                |
                                v
+----------------------------------------------------------------+
| 8. 视频处理                                                     |
|                                                                |
|    RTP -> FU-A/STAP-A/Single NAL -> Annex-B                    |
|        -> SPS/PPS -> 等待 IDR -> 解码 -> 显示                  |
+-------------------------------+--------------------------------+
                                |
                                v
+----------------------------------------------------------------+
| 9. 挂断                                                         |
|                                                                |
|    BYE -> 200 OK                                                |
|    UA_EVENT_CALL_CLOSED                                        |
|    释放音频、视频和会话资源                                    |
+----------------------------------------------------------------+
```

---

## 15. Baresip 事件与网络报文对应关系

### 15.1 主动呼出

```text
ua_connect(..., VIDMODE_OFF)
        |
        v
UA_EVENT_CALL_OUTGOING
        |
        v
UA_EVENT_CALL_LOCAL_SDP("offer")
        |
        v
发送 INVITE + SDP Offer
        |
        +--> 收到 180 Ringing
        |       |
        |       v
        |   UA_EVENT_CALL_RINGING
        |
        +--> 收到 183 + SDP（可能存在）
        |       |
        |       v
        |   UA_EVENT_CALL_PROGRESS
        |
        v
收到 200 OK + SDP Answer
        |
        +--> UA_EVENT_CALL_ANSWERED
        +--> UA_EVENT_CALL_REMOTE_SDP("answer")
        |
        v
发送 ACK
        |
        v
UA_EVENT_CALL_ESTABLISHED
        |
        v
收到音频 RTP
        |
        v
UA_EVENT_CALL_RTPESTAB("audio")
```

### 15.2 接听来电

```text
收到 INVITE + SDP Offer
        |
        +--> UA_EVENT_CALL_INCOMING
        +--> UA_EVENT_CALL_REMOTE_SDP("offer")
        |
        v
用户接听
        |
        v
ua_answer(...)
        |
        v
UA_EVENT_CALL_LOCAL_SDP("answer")
        |
        v
发送 200 OK + SDP Answer
        |
        v
收到 ACK
        |
        v
UA_EVENT_CALL_ESTABLISHED
        |
        v
收到音频/视频 RTP
        |
        +--> UA_EVENT_CALL_RTPESTAB("audio")
        +--> UA_EVENT_CALL_RTPESTAB("video")
```

事件顺序可能因早期媒体、重传、网络质量和 Offer/Answer 方式发生变化，不能将事件列表当成绝对线性的状态机。

---

## 16. 仓库历史抓包说明

仓库 `docs/待处理.md` 中保存了一段历史 SIP 抓包。远端发出了：

```text
INVITE sip:192.168.17.154:5060 SIP/2.0
```

携带音频 SDP：

```text
m=audio 40481 RTP/AVP 96 0 8 101 97
a=rtpmap:96 opus/48000/2
a=rtpmap:101 telephone-event/48000
a=rtpmap:97 telephone-event/8000
```

但该次呼叫最终返回：

```text
SIP/2.0 404 Not Found
```

因此它不是一次成功通话，不能用它确认成功通话中的：

- 本机 `200 OK` SDP Answer；
- 最终选择的 G.711 codec；
- 实际音视频 RTP 端口；
- RTP Sequence、Timestamp 和 SSRC；
- BYE 结束流程。

本文的成功通话报文是根据当前代码配置、Baresip 行为和标准 SIP/SDP/RTP 结构重建的。运行时动态字段应以设备上的 tcpdump 或 Wireshark 抓包为准。

---

## 17. 建议抓包方式

仅抓 SIP：

```bash
tcpdump -ni any -s0 -vvv -A 'udp port 5060'
```

抓当前项目 SIP 和 RTP 端口范围：

```bash
tcpdump -ni any -s0 -w /tmp/sip_rtp_call.pcap 'udp port 5060 or udp portrange 10000-10100'
```

使用 Wireshark 打开后可按以下过滤器检查：

```text
sip
sdp
rtp
rtcp
sip || rtp || rtcp
```

如果 Wireshark 没有自动识别动态 RTP 端口，可以使用 SDP 中的 `m=audio`、`m=video` 端口，或者执行 `Decode As -> RTP`。

检查一通成功视频来电时，应重点确认：

1. INVITE 中是否存在 `m=audio` 和 `m=video`；
2. H.264 PT 与 `a=rtpmap` 是否一致；
3. Offer/Answer 的 `packetization-mode` 是否匹配；
4. 本机 Answer 是否为 `audio=sendrecv`、`video=recvonly`；
5. RTP 目的 IP/端口是否与对端 SDP 一致；
6. 音频 RTP PT 是否为最终协商的 PCMU 或 PCMA；
7. 视频 RTP Sequence 是否连续；
8. FU-A Start/Middle/End 是否完整；
9. SPS/PPS 是否来自 SDP 或 RTP；
10. SIP 建立后是否出现 `UA_EVENT_CALL_RTPESTAB("audio"/"video")`。
