# MiniVisor on ODROID M1S

MiniVisorを[ODROID M1S](https://www.hardkernel.com/shop/odroid-m1s-with-8gbyte-ram/)で動作させるための手順

## U-Boot Build & Install

U-BootのソースコードとしてU-Boot公式のリポジトリとHardKernelが提供しているリポジトリがあります。
基本的には前者がおすすめです。

ODROIDに刺すSDカードは `/dev/mmcblk0` として認識されていることを前提としています。
SDカードのフォーマットは `sudo fdisk /dev/mmcblk0` で以下のとおりに操作します。

```
コマンド (m でヘルプ): o
新しい DOS (MBR) ディスクラベルを作成しました。識別子は 0x75588992 です。
デバイスには既に 'gpt' 署名が書き込まれており、write (書き込み)コマンドを実行すると消えてしまいます。詳しくは、 fdisk(8) man ページ、 --wipe オプションを参照してください。

コマンド (m でヘルプ): n
パーティションタイプ
   p   基本パーティション (0 プライマリ, 0 拡張, 4 空き)
   e   拡張領域 (論理パーティションが入ります)
選択 (既定値 p):

既定の回答 p であるものとみなします。
パーティション番号 (1-4, 既定値 1):
最初のセクタ (2048-60973055, 既定値 2048):
最終セクタ, +/-セクタ番号 または +/-サイズ{K,M,G,T,P} (2048-60973055, 既定値 60973055): +4096

新しいパーティション 1 をタイプ Linux、サイズ 2 MiB で作成しました。

コマンド (m でヘルプ): n
パーティションタイプ
   p   基本パーティション (1 プライマリ, 0 拡張, 3 空き)
   e   拡張領域 (論理パーティションが入ります)
選択 (既定値 p):

既定の回答 p であるものとみなします。
パーティション番号 (2-4, 既定値 2):
最初のセクタ (6145-60973055, 既定値 8192):
最終セクタ, +/-セクタ番号 または +/-サイズ{K,M,G,T,P} (8192-60973055, 既定値 60973055):

新しいパーティション 2 をタイプ Linux、サイズ 29.1 GiB で作成しました。

コマンド (m でヘルプ): t
パーティション番号 (1,2, 既定値 2): 1
16 進数コード または別名 (L で利用可能なコードを一覧表示します): a2

パーティションのタイプを 'Linux' から '不明' に変更しました。

コマンド (m でヘルプ): t
パーティション番号 (1,2, 既定値 2): 2
16 進数コード または別名 (L で利用可能なコードを一覧表示します): c

パーティションのタイプを 'Linux' から 'W95 FAT32 (LBA)' に変更しました。

コマンド (m でヘルプ): w
パーティション情報が変更されました。
ioctl() を呼び出してパーティション情報を再読み込みします。
ディスクを同期しています。
```

### U-Boot Official

```bash
git clone --depth 1 https://source.denx.de/u-boot/u-boot.git -b v2025.10
git clone --depth 1 https://github.com/rockchip-linux/rkbin.git
cd u-boot
make odroid-m1s-rk3566_defconfig
CROSS_COMPILE=aarch64-linux-gnu-  ROCKCHIP_TPL=../rkbin/bin/rk35/rk3566_ddr_1056MHz_v1.23.bin BL31=../rkbin/bin/rk35/rk3568_bl31_v1.45.elf make -j$(nproc)

# Write into the SD Card
sudo dd if=idbloader.img of=/dev/mmcblk0 seek=64
sudo dd if=u-boot.itb of=/dev/mmcblk0p1
```

### HardKernel Repo

基本的には https://wiki.odroid.com/odroid-m1s/board_support/building_u-boot#checkout_compile に沿って作業します。

ただし、リポジトリの内容が古いためPython3環境で動かすには以下のパッチの適用が必要です。

```diff
diff --git a/arch/arm/mach-rockchip/decode_bl31.py b/arch/arm/mach-rockchip/decode_bl31.py
index 301bd15375..37c7306661 100755
--- a/arch/arm/mach-rockchip/decode_bl31.py
+++ b/arch/arm/mach-rockchip/decode_bl31.py
@@ -1,4 +1,4 @@
-#!/usr/bin/env python2
+#!/usr/bin/env python
 #
 # Copyright (C) 2020 Rockchip Electronics Co., Ltd
 #
diff --git a/arch/arm/mach-rockchip/make_fit_atf.py b/arch/arm/mach-rockchip/make_fit_atf.py
index 27b6ef7597..0551d95ec1 100755
--- a/arch/arm/mach-rockchip/make_fit_atf.py
+++ b/arch/arm/mach-rockchip/make_fit_atf.py
@@ -1,4 +1,4 @@
-#!/usr/bin/env python2
+#!/usr/bin/env python
 """
 A script to generate FIT image source for rockchip boards
 with ARM Trusted Firmware
diff --git a/common/command.c b/common/command.c
index 7171557265..001238a9fc 100644
--- a/common/command.c
+++ b/common/command.c
@@ -501,7 +501,7 @@ static int cmd_call(cmd_tbl_t *cmdtp, int flag, int argc, char * const argv[])
        return result;
 }

-enum command_ret_t cmd_process(int flag, int argc, char * const argv[],
+int cmd_process(int flag, int argc, char * const argv[],
                               int *repeatable, ulong *ticks)
 {
        enum command_ret_t rc = CMD_RET_SUCCESS;
diff --git a/configs/odroid_rk3566_defconfig b/configs/odroid_rk3566_defconfig
index 29e757503b..ccbd6f98c5 100644
--- a/configs/odroid_rk3566_defconfig
+++ b/configs/odroid_rk3566_defconfig
@@ -538,7 +538,7 @@ CONFIG_CMD_BOOTEFI=y
 # CONFIG_CMD_BOOTEFI_HELLO_COMPILE is not set
 # CONFIG_CMD_BOOTMENU is not set
 CONFIG_CMD_DTIMG=y
-# CONFIG_CMD_ELF is not set
+CONFIG_CMD_ELF=y
 CONFIG_CMD_FDT=y
 CONFIG_CMD_GO=y
 CONFIG_CMD_RUN=y
diff --git a/include/linux/compiler-gcc.h b/include/linux/compiler-gcc.h
index 810dbeb503..3850c2cd5b 100644
--- a/include/linux/compiler-gcc.h
+++ b/include/linux/compiler-gcc.h
@@ -197,7 +197,6 @@
  * this in the preprocessor, but we can live with this because they're
  * unreleased.  Really, we need to have autoconf for the kernel.
  */
-#define unreachable() __builtin_unreachable()

 /* Mark a function definition as prohibited from being cloned. */
 #define __noclone      __attribute__((__noclone__))
diff --git a/make.sh b/make.sh
index 69f294a9df..c0a290fcc2 100755
--- a/make.sh
+++ b/make.sh
@@ -733,7 +733,7 @@ function pack_fit_image()
                echo "ERROR: No 'dtc', please: apt-get install device-tree-compiler"
                exit 1
        elif [ "${ARM64_TRUSTZONE}" == "y" ]; then
-               if ! which python2 >/dev/null 2>&1 ; then
+               if ! which python3 >/dev/null 2>&1 ; then
                        echo "ERROR: No python2"
                        exit 1
                fi
@@ -800,7 +800,7 @@ select_ini_file
 handle_args_late
 sub_commands
 clean_files
-make PYTHON=python2 ${ARG_SPL_FWVER} ${ARG_FWVER} CROSS_COMPILE=${TOOLCHAIN} all --jobs=${JOB}
+make PYTHON=python ${ARG_SPL_FWVER} ${ARG_FWVER} CROSS_COMPILE=${TOOLCHAIN} all --jobs=${JOB}
 pack_idblock
 pack_images
 finish
 ```

## RAM Layout

使用するU-BootのソースコードによってDRAMのマップされている位置が違います。
必要に応じて`src/main.rs`と`scripts/m1s.ld`を設定します。

以下は8GBモデルの場合です。

### U-Boot

- RAM1: 0x000200000 ~ 0x0F0000000 (Size: 0x0EFE00000)
- RAM2: 0x100000000 ~ 0x200000000 (Size: 0x100000000)

m1s.ld: `__RAM_BASE   = 0x200000;`

### HardKernel

- RAM1: 0x000200000 ~ 0x008400000 (Size: 0x08200000)
- RAM2: 0x009400000 ~ 0x0F0000000 (Size: 0xE6C00000)

m1s.ld : `__RAM_BASE   = 0x280000;`

## SD Card Layout

`/dev/mmcblk0p1`をFAT32でフォーマットした後マウントしてください。

中は以下のような構造になっています。

```
.
├── DISK0
├── DISK1
├── DTB
├── Image
├── mini_visor
└── u-boot.dtb
```

各ファイルは以下の位置から取得してください。

- `DISK0` / `DISK1`: 書籍を参照してください
- `DTB`: 書籍を参照してください
- `Image`: 書籍を参照してください
- `mini_visor`: `cargo build --release`でビルド後`MiniVisor/target/aarch64-unknown-none-softfloat/release/mini_visor`
- `u-boot.dtb`
    - Official: `u-boot/u-boot.dtb`
    - HardKernel: `arch/arm/dts/rk3566-odroid-m1s.dtb`

## Boot

SDカードを挿入して起動後、[Debug UART](https://wiki.odroid.com/odroid-m1s/board_support/uart_debugging)で以下のように操作してください。

```
setenv autostart yes
fatload mmc 1:2 $kernel_addr_r mini_visor
fatload mmc 1:2 $fdt_addr_r u-boot.dtb
bootelf $kernel_addr_r $fdt_addr_r $kernel_addr_r
```
