# SSD20X 交叉编译 tcpdump

本文档记录如何使用当前项目的 SSD202D/SSD20X ARM 工具链，交叉编译可在设备上单文件运行的静态版 `tcpdump`。

本文步骤已经在当前构建环境中实际执行并验证，最终生成：

```text
tcpdump 4.99.4
libpcap 1.10.4（支持 TPACKET_V3）
ARMv7 / EABI5 / hard-float
静态链接，无动态加载器依赖
```

## 1. 编译环境

### 1.1 目标平台

当前项目使用的目标平台为：

- 芯片：SSD202D / SSD20X；
- CPU 架构：ARMv7-A；
- 浮点 ABI：hard-float；
- C 库：工具链自带 glibc；
- 交叉编译器前缀：`arm-linux-gnueabihf-`。

项目工具链配置见：

```text
cmake/toolchain/ssd202.cmake
```

默认工具链目录为：

```text
/opt/toolchain/gcc-arm-8.2-2018.08-x86_64-arm-linux-gnueabihf
```

编译前检查工具链：

```bash
/opt/toolchain/gcc-arm-8.2-2018.08-x86_64-arm-linux-gnueabihf/bin/arm-linux-gnueabihf-gcc --version
/opt/toolchain/gcc-arm-8.2-2018.08-x86_64-arm-linux-gnueabihf/bin/arm-linux-gnueabihf-gcc -dumpmachine
/opt/toolchain/gcc-arm-8.2-2018.08-x86_64-arm-linux-gnueabihf/bin/arm-linux-gnueabihf-gcc -print-sysroot
```

预期目标三元组为：

```text
arm-linux-gnueabihf
```

### 1.2 安装主机端构建工具

在 Ubuntu/WSL 中执行：

```bash
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    flex \
    bison \
    pkg-config \
    curl
```

`flex` 和 `bison` 只在编译主机上运行，不会成为设备端依赖。

如果需要在 x86 主机上运行 ARM 程序进行启动验证，可额外安装：

```bash
sudo apt-get install -y qemu-user
```

## 2. 准备目录和源码

以下命令均从项目根目录执行：

```bash
cd /data/code/source
```

创建构建目录：

```bash
mkdir -p build/tcpdump-ssd20x/downloads
mkdir -p build/tcpdump-ssd20x/src
mkdir -p build/tcpdump-ssd20x/install
mkdir -p build/tcpdump-ssd20x/output
```

下载官方发布包：

```bash
curl --fail --location --retry 2 \
    --output build/tcpdump-ssd20x/downloads/libpcap-1.10.4.tar.gz \
    https://www.tcpdump.org/release/libpcap-1.10.4.tar.gz

curl --fail --location --retry 2 \
    --output build/tcpdump-ssd20x/downloads/tcpdump-4.99.4.tar.gz \
    https://www.tcpdump.org/release/tcpdump-4.99.4.tar.gz
```

解压源码：

```bash
rm -rf build/tcpdump-ssd20x/src/libpcap-1.10.4
rm -rf build/tcpdump-ssd20x/src/tcpdump-4.99.4

tar -xzf build/tcpdump-ssd20x/downloads/libpcap-1.10.4.tar.gz \
    -C build/tcpdump-ssd20x/src

tar -xzf build/tcpdump-ssd20x/downloads/tcpdump-4.99.4.tar.gz \
    -C build/tcpdump-ssd20x/src
```

解压后的目录结构为：

```text
build/tcpdump-ssd20x/
├── downloads/
├── install/
├── output/
└── src/
    ├── libpcap-1.10.4/
    └── tcpdump-4.99.4/
```

## 3. 设置交叉编译环境

在当前终端设置：

```bash
export PROJECT_ROOT=/data/code/source
export BUILD_ROOT=$PROJECT_ROOT/build/tcpdump-ssd20x
export PREFIX=$BUILD_ROOT/install

export TOOLCHAIN_ROOT=/opt/toolchain/gcc-arm-8.2-2018.08-x86_64-arm-linux-gnueabihf
export CROSS_COMPILE=$TOOLCHAIN_ROOT/bin/arm-linux-gnueabihf-

export CC=${CROSS_COMPILE}gcc
export AR=${CROSS_COMPILE}ar
export RANLIB=${CROSS_COMPILE}ranlib
export STRIP=${CROSS_COMPILE}strip
export READELF=${CROSS_COMPILE}readelf
export HOST=arm-linux-gnueabihf
```

确认静态 libc 存在：

```bash
ls -l $TOOLCHAIN_ROOT/arm-linux-gnueabihf/libc/usr/lib/libc.a
ls -l $TOOLCHAIN_ROOT/arm-linux-gnueabihf/libc/usr/lib/libpthread.a
```

缺少这些静态库时，无法生成当前文档中的完全静态版本。

## 4. 交叉编译静态 libpcap

进入 `libpcap` 源码目录：

```bash
cd $BUILD_ROOT/src/libpcap-1.10.4
```

配置：

```bash
CC="$CC" \
AR="$AR" \
RANLIB="$RANLIB" \
CFLAGS="-Os -ffunction-sections -fdata-sections" \
LDFLAGS="-Wl,--gc-sections" \
./configure \
    --host="$HOST" \
    --prefix="$PREFIX" \
    --with-pcap=linux \
    --disable-shared \
    --enable-static \
    --without-libnl \
    --disable-dbus \
    --disable-usb \
    --disable-bluetooth
```

参数说明：

- `--with-pcap=linux`：使用 Linux Packet Socket 抓包后端；
- `--disable-shared --enable-static`：只生成静态 `libpcap.a`；
- `--without-libnl`：不依赖设备上的 libnl；
- `--disable-dbus`：不依赖 D-Bus；
- `--disable-usb`：不构建 USB 抓包支持；
- `--disable-bluetooth`：不构建蓝牙抓包支持；
- `-Os`：优先减小程序体积；
- `--gc-sections`：链接时删除未使用的代码段。

编译并安装到临时目录：

```bash
make -j8
make install
```

检查静态库：

```bash
ls -lh $PREFIX/lib/libpcap.a
$AR t $PREFIX/lib/libpcap.a
```

## 5. 交叉编译静态 tcpdump

进入 `tcpdump` 源码目录：

```bash
cd $BUILD_ROOT/src/tcpdump-4.99.4
```

配置：

```bash
PATH="$PREFIX/bin:$PATH" \
PKG_CONFIG_LIBDIR="$PREFIX/lib/pkgconfig" \
PCAP_CONFIG="$PREFIX/bin/pcap-config" \
CC="$CC" \
AR="$AR" \
RANLIB="$RANLIB" \
CPPFLAGS="-I$PREFIX/include" \
CFLAGS="-Os -ffunction-sections -fdata-sections" \
LDFLAGS="-L$PREFIX/lib -static -Wl,--gc-sections" \
./configure \
    --host="$HOST" \
    --prefix="$PREFIX" \
    --without-crypto \
    --disable-smb
```

参数说明：

- `PKG_CONFIG_LIBDIR`：只使用交叉编译生成的 `libpcap` 信息，避免错误链接主机 x86 库；
- `-static`：静态链接 libc 和 `libpcap`，设备上不需要额外复制 `.so`；
- `--without-crypto`：不引入 OpenSSL 等加密库依赖；
- `--disable-smb`：关闭不需要的 SMB 解析支持，减少依赖和体积。

开始编译：

```bash
make -j8
```

成功后生成：

```text
build/tcpdump-ssd20x/src/tcpdump-4.99.4/tcpdump
```

链接阶段可能出现以下 glibc 警告：

```text
Using 'getaddrinfo' in statically linked applications requires at runtime
shared libraries from the glibc version used for linking
```

这是 glibc 静态链接名称解析接口时的常见提示，不表示链接失败。设备抓包时建议始终使用 `-nn`，禁止 DNS 和服务名称解析。

## 6. 裁剪并生成最终文件

保留一份未裁剪程序，方便后续调试：

```bash
cp $BUILD_ROOT/src/tcpdump-4.99.4/tcpdump \
    $BUILD_ROOT/output/tcpdump-4.99.4-ssd20x-armhf-static.unstripped
```

生成裁剪后的部署文件：

```bash
$STRIP \
    -o $BUILD_ROOT/output/tcpdump-4.99.4-ssd20x-armhf-static \
    $BUILD_ROOT/src/tcpdump-4.99.4/tcpdump

chmod 755 $BUILD_ROOT/output/tcpdump-4.99.4-ssd20x-armhf-static
```

生成 SHA-256 校验文件：

```bash
cd $BUILD_ROOT
sha256sum output/tcpdump-4.99.4-ssd20x-armhf-static \
    > output/tcpdump-4.99.4-ssd20x-armhf-static.sha256
```

本次实际编译的裁剪版大小约为 `1.5 MB`。

## 7. 校验目标程序

### 7.1 检查 ELF 架构

```bash
$READELF -h $BUILD_ROOT/output/tcpdump-4.99.4-ssd20x-armhf-static
```

关键结果应包含：

```text
Class:                             ELF32
Machine:                           ARM
Flags:                             Version5 EABI, hard-float ABI
```

检查 ARM 属性：

```bash
$READELF -A $BUILD_ROOT/output/tcpdump-4.99.4-ssd20x-armhf-static
```

当前工具链生成的关键属性为：

```text
Tag_CPU_arch: v7
Tag_FP_arch: VFPv3
Tag_Advanced_SIMD_arch: NEONv1
Tag_ABI_VFP_args: VFP registers
```

### 7.2 确认没有动态加载器依赖

```bash
$READELF -l $BUILD_ROOT/output/tcpdump-4.99.4-ssd20x-armhf-static
```

完全静态程序的 Program Headers 中不应出现：

```text
INTERP
DYNAMIC
```

### 7.3 使用 QEMU 进行启动验证

安装 `qemu-user` 后执行：

```bash
qemu-arm $BUILD_ROOT/output/tcpdump-4.99.4-ssd20x-armhf-static --version
```

本次实际验证结果：

```text
tcpdump-4.99.4-ssd20x-armhf-static version 4.99.4
libpcap version 1.10.4 (with TPACKET_V3)
```

QEMU 启动验证只能证明程序能够在 ARM 用户态运行。抓取真实网卡仍需复制到 SSD20X 设备上测试。

## 8. 复制到项目 tool 目录

当前 `tool/` 已在 `.gitignore` 中忽略，适合临时存放部署工具。

从项目根目录执行：

```bash
cp build/tcpdump-ssd20x/output/tcpdump-4.99.4-ssd20x-armhf-static \
    tool/tcpdump-4.99.4

chmod 755 tool/tcpdump-4.99.4
```

注意，本地文件复制使用 `cp`，不是 `scp`。

## 9. 部署到 SSD20X 设备

从开发机复制到设备：

```bash
scp tool/tcpdump-4.99.4 root@设备IP:/tmp/tcpdump
```

设备上设置可执行权限并检查版本：

```sh
chmod +x /tmp/tcpdump
/tmp/tcpdump --version
/tmp/tcpdump -D
```

抓取 `eth0`：

```sh
/tmp/tcpdump -i eth0 -nn -s 0
```

抓取所有网卡并保存到文件：

```sh
/tmp/tcpdump -i any -nn -s 0 -w /tmp/capture.pcap
```

将抓包文件复制回开发机：

```bash
scp root@设备IP:/tmp/capture.pcap ./capture.pcap
```

之后可使用 Wireshark 打开 `capture.pcap`。

## 10. 常用抓包命令

抓取 SIP 端口：

```sh
/tmp/tcpdump -ni any -s 0 'udp port 5060 or tcp port 5060 or tcp port 5061'
```

抓取指定主机的 SIP 数据：

```sh
/tmp/tcpdump -ni any -s 0 -vvv -A \
    'host 192.168.17.236 and udp port 5060'
```

抓取指定 TCP 端口：

```sh
/tmp/tcpdump -i eth0 -nn -s 0 tcp port 80
```

抓取指定主机：

```sh
/tmp/tcpdump -i eth0 -nn -s 0 host 192.168.1.100
```

## 11. 设备内核要求

`tcpdump` 依赖 Linux Packet Socket，SSD20X 内核必须启用：

```text
CONFIG_PACKET=y
```

设备支持 `/proc/config.gz` 时可检查：

```sh
zcat /proc/config.gz | grep CONFIG_PACKET
```

也可以在 SSD20X SDK 的内核 `.config` 中检查：

```bash
grep CONFIG_PACKET path/to/kernel/.config
```

如果运行时出现：

```text
socket: Address family not supported by protocol
```

通常说明内核没有启用 `CONFIG_PACKET`，需要重新配置并编译内核。

抓包还需要 root 权限，或者具备 `CAP_NET_RAW`、`CAP_NET_ADMIN` capability。SSD20X 精简系统通常直接使用 root 执行。

## 12. 常见问题

### 12.1 提示源文件不存在

错误示例：

```text
cp: cannot stat 'build/tcpdump-ssd20x/output/tcpdump-4.99.4': No such file or directory
```

原因是最终文件的完整名称为：

```text
build/tcpdump-ssd20x/output/tcpdump-4.99.4-ssd20x-armhf-static
```

正确复制命令：

```bash
cp build/tcpdump-ssd20x/output/tcpdump-4.99.4-ssd20x-armhf-static \
    tool/tcpdump-4.99.4
```

### 12.2 configure 提示找不到 flex 或 lex

错误：

```text
configure: error: Neither flex nor lex was found
```

安装主机端依赖：

```bash
sudo apt-get install -y flex bison
```

然后重新执行 `libpcap` 的 `configure` 和 `make`。

### 12.3 文件存在但设备提示 not found

如果设备执行动态程序时出现：

```text
./tcpdump: not found
```

但文件实际存在，通常表示动态加载器或 libc ABI 不匹配。本文件生成的是完全静态版本，不依赖设备上的 `/lib/ld-linux-armhf.so.3`，可以避免此问题。

仍需确认设备 CPU 支持 ARMv7 hard-float；如果设备 RootFS 使用 soft-float，则必须换成匹配的 `arm-linux-gnueabi-` 工具链重新编译。

### 12.4 Permission denied

先设置执行权限：

```sh
chmod +x /tmp/tcpdump
```

如果仍然失败，检查所在文件系统是否使用了 `noexec` 挂载选项，可换到允许执行的目录。

### 12.5 无法抓取网卡

依次检查：

```sh
/tmp/tcpdump -D
ip link
cat /proc/net/dev
```

确认网卡名称正确，并使用 root 权限运行。若内核不支持 Packet Socket，还需要启用 `CONFIG_PACKET=y`。
